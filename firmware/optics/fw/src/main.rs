//! The optics controller's firmware. All its decisions are in `optics-core`: this wires that to the
//! carrier board's pins and to USB.
//!
//! The control loop owns the pins. USB talks to it only through queues, and it never waits for
//! one, so a PC that has stopped reading can't hold up the door's interlock.
#![no_std]
#![no_main]
#![deny(unsafe_code)]

use embassy_executor::Spawner;
use embassy_futures::join::join3;
use embassy_futures::select::{Either4, select, select4};
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::{SPI1, USB};
use embassy_rp::spi::{self, Spi};
use embassy_rp::uart::{self, Blocking, Uart, UartRx, UartTx};
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{Peri, binary_info, bind_interrupts};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Instant, Timer};
use embassy_usb::class::cdc_acm::{CdcAcmClass, Receiver, Sender, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config};
use embedded_hal_nb::nb;
use embedded_hal_nb::serial::Read as _;
use optics_core::tmc2209::{self, DriverWire};
use optics_core::{
    CAMERAS, CHANNELS, Controller, Event, Inputs, Line, LineBuffer, Pattern, Request,
};

/// The most the control loop sleeps, so the door is read this often even if no edge wakes it.
const TICK: Duration = Duration::from_millis(10);
/// A control loop that stalls for this long resets the chip, which puts out the LEDs.
const WATCHDOG: Duration = Duration::from_millis(500);
const USB_PACKET: u16 = 64;
/// The matrix's cable is unshielded, and its run to the moving lift long, so the clock stays low.
const MATRIX_CLOCK_HZ: u32 = 4_000_000;
/// How long a step pulse stays high, and then low: 1 µs at the chip's 150 MHz.
const STEP_PULSE_CYCLES: u32 = 150;
/// The longest a whole exchange with the slider's driver should take: 12 bytes at 115,200 baud is
/// about 1 ms.
const DRIVER_TIMEOUT: Duration = Duration::from_millis(3);

/// pid.codes' VID, and its PID for testing (ADR-0050).
const USB_IDS: (u16, u16) = (0x1209, 0x0001);

static REQUESTS: Channel<CriticalSectionRawMutex, Request, 4> = Channel::new();
static LINES_OUT: Channel<CriticalSectionRawMutex, Line, 16> = Channel::new();

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

// What `picotool info` shows.
#[allow(unsafe_code)]
// SAFETY: Nothing else is placed in this section, and only picotool reads it. memory.x keeps it in
// flash, between the symbols that the binary-info header points to.
#[unsafe(link_section = ".bi_entries")]
#[used]
static PICOTOOL_ENTRIES: [binary_info::EntryAddr; 3] = [
    binary_info::rp_program_name!(c"OPTIMA optics controller"),
    binary_info::rp_cargo_version!(),
    binary_info::rp_program_url!(c"https://github.com/lucaswewa/microscope"),
];

/// Resetting returns every pin to its default, and the carrier board's pull-downs put out the LEDs.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    cortex_m::peripheral::SCB::sys_reset()
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // The pins are in docs/misc/optima_carrier_board.md, section 2. Each input has its own pull
    // resistor on the board, and the RP2350's erratum E9 is a reason not to use the internal ones.
    let mut matrix_config = spi::Config::default();
    matrix_config.frequency = MATRIX_CLOCK_HZ;
    let mut pins = Pins {
        leds: [
            Output::new(p.PIN_12, Level::Low),
            Output::new(p.PIN_13, Level::Low),
            Output::new(p.PIN_14, Level::Low),
            Output::new(p.PIN_15, Level::Low),
        ],
        triggers: [
            Output::new(p.PIN_16, Level::Low),
            Output::new(p.PIN_18, Level::Low),
        ],
        port_set: Output::new(p.PIN_20, Level::Low),
        slider_step: Output::new(p.PIN_2, Level::Low),
        slider_dir: Output::new(p.PIN_3, Level::Low),
        slider_enable: Output::new(p.PIN_4, Level::High),
        matrix: Spi::new_blocking_txonly(p.SPI1, p.PIN_10, p.PIN_11, matrix_config),
        matrix_shown: Pattern::OFF,
    };
    // The panel keeps its last pattern through a reset of this chip.
    let _ = pins.matrix.blocking_write(&Pattern::OFF.frame());

    let mut door = Input::new(p.PIN_6, Pull::None);
    let mut exposure_1 = Input::new(p.PIN_17, Pull::None);
    let mut exposure_2 = Input::new(p.PIN_19, Pull::None);
    let port_at = Input::new(p.PIN_21, Pull::None);
    let slider_home = Input::new(p.PIN_5, Pull::None);

    // 115,200 baud, 8N1, which the driver detects by itself.
    let (tx, rx) = Uart::new_blocking(p.UART0, p.PIN_0, p.PIN_1, uart::Config::default()).split();
    let mut driver = DriverUart { tx, rx };

    let mut watchdog = Watchdog::new(p.WATCHDOG);
    watchdog.start(WATCHDOG);
    spawner.spawn(usb_task(p.USB).unwrap());

    let mut controller = Controller::new();
    loop {
        let now = Instant::now().as_micros();
        let inputs = Inputs {
            door_shut: door.is_high(),
            exposure_low: [exposure_1.is_low(), exposure_2.is_low()],
            port_at: port_at.is_high(),
            slider_home: slider_home.is_high(),
        };
        controller.poll(now, inputs, |event: Event| {
            LINES_OUT.try_send(event.to_line()).is_ok()
        });
        if controller.slider_wants_driver() {
            // A few milliseconds, once per homing, and the door's hardware doesn't depend on it.
            let ok = tmc2209::configure(&mut driver);
            controller.driver_configured(ok, Instant::now().as_micros());
        }
        pins.set(&mut controller);
        watchdog.feed(WATCHDOG);

        let tick = now + TICK.as_micros();
        let wake = controller
            .next_deadline_us()
            .map_or(tick, |due| due.min(tick));
        let woken = select4(
            REQUESTS.receive(),
            door.wait_for_any_edge(),
            select(
                exposure_1.wait_for_any_edge(),
                exposure_2.wait_for_any_edge(),
            ),
            Timer::at(Instant::from_micros(wake)),
        )
        .await;
        if let Either4::First(request) = woken {
            let reply = match request {
                Request::Line(line) => {
                    Some(controller.command(line.as_str(), Instant::now().as_micros()))
                }
                Request::Rejected => Some(Controller::rejected_line()),
                Request::HostLost => {
                    controller.host_lost();
                    None
                }
            };
            if let Some(reply) = reply
                && LINES_OUT.try_send(reply).is_err()
            {
                controller.count_dropped();
            }
            pins.set(&mut controller);
        }
    }
}

/// Everything the control loop drives.
struct Pins<'d> {
    leds: [Output<'d>; CHANNELS],
    triggers: [Output<'d>; CAMERAS],
    port_set: Output<'d>,
    slider_step: Output<'d>,
    slider_dir: Output<'d>,
    slider_enable: Output<'d>,
    matrix: Spi<'d, SPI1, spi::Blocking>,
    matrix_shown: Pattern,
}

impl Pins<'_> {
    /// The matrix goes last, because its frame holds up the loop for about half a millisecond.
    fn set(&mut self, controller: &mut Controller) {
        let outputs = controller.outputs();
        for (pin, on) in self.leds.iter_mut().zip(outputs.leds) {
            pin.set_level(Level::from(on));
        }
        for (pin, on) in self.triggers.iter_mut().zip(outputs.triggers) {
            pin.set_level(Level::from(on));
        }
        self.port_set.set_level(Level::from(outputs.port_set));
        self.slider_enable
            .set_level(Level::from(!outputs.slider_on));
        self.slider_dir.set_level(Level::from(outputs.slider_dir));
        for _ in 0..controller.take_steps() {
            self.slider_step.set_high();
            cortex_m::asm::delay(STEP_PULSE_CYCLES);
            self.slider_step.set_low();
            cortex_m::asm::delay(STEP_PULSE_CYCLES);
        }
        if outputs.matrix != self.matrix_shown {
            let _ = self.matrix.blocking_write(&outputs.matrix.frame());
            self.matrix_shown = outputs.matrix;
        }
    }
}

/// The slider's driver, on one wire: this chip hears its own bytes before any reply.
struct DriverUart<'d> {
    tx: UartTx<'d, Blocking>,
    rx: UartRx<'d, Blocking>,
}

impl DriverWire for DriverUart<'_> {
    fn exchange(&mut self, send: &[u8], reply: &mut [u8]) -> bool {
        while !matches!(self.rx.read(), Err(nb::Error::WouldBlock)) {}
        if self.tx.blocking_write(send).is_err() {
            return false;
        }
        let deadline = Instant::now() + DRIVER_TIMEOUT;
        for index in 0..send.len() + reply.len() {
            let byte = loop {
                match self.rx.read() {
                    Ok(byte) => break byte,
                    Err(nb::Error::WouldBlock) if Instant::now() < deadline => {}
                    Err(_) => return false,
                }
            };
            match send.get(index) {
                Some(sent) if *sent != byte => return false,
                Some(_) => {}
                None => reply[index - send.len()] = byte,
            }
        }
        true
    }
}

#[embassy_executor::task]
async fn usb_task(mut usb: Peri<'static, USB>) {
    let mut config = Config::new(USB_IDS.0, USB_IDS.1);
    config.manufacturer = Some("OPTIMA");
    config.product = Some("OPTIMA optics controller");

    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control = [0; 64];
    let mut state = State::new();
    let mut builder = Builder::new(
        Driver::new(usb.reborrow(), Irqs),
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [],
        &mut control,
    );
    let class = CdcAcmClass::new(&mut builder, &mut state, USB_PACKET);
    let mut device = builder.build();
    let (mut sender, mut receiver) = class.split();

    join3(
        device.run(),
        read_lines(&mut receiver),
        write_lines(&mut sender),
    )
    .await;
}

async fn read_lines<'d>(receiver: &mut Receiver<'d, Driver<'d, USB>>) {
    let mut buffer = LineBuffer::new();
    let mut packet = [0; USB_PACKET as usize];
    loop {
        receiver.wait_connection().await;
        while let Ok(count) = receiver.read_packet(&mut packet).await {
            for &byte in &packet[..count] {
                if let Some(request) = buffer.push(byte) {
                    REQUESTS.send(request).await;
                }
            }
        }
        buffer.clear();
        REQUESTS.send(Request::HostLost).await;
    }
}

async fn write_lines<'d>(sender: &mut Sender<'d, Driver<'d, USB>>) {
    loop {
        sender.wait_connection().await;
        while send_line(sender, LINES_OUT.receive().await).await.is_ok() {}
    }
}

/// Sends a line in packets, with an empty one after a full one so that the host doesn't wait.
async fn send_line<'d>(
    sender: &mut Sender<'d, Driver<'d, USB>>,
    mut line: Line,
) -> Result<(), EndpointError> {
    let bytes = line.to_send();
    for packet in bytes.chunks(USB_PACKET as usize) {
        sender.write_packet(packet).await?;
    }
    if bytes.len().is_multiple_of(USB_PACKET as usize) {
        sender.write_packet(&[]).await?;
    }
    Ok(())
}
