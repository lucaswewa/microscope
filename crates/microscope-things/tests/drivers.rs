//! The device-actor pattern (ADR-0041), with a driver over a scripted port:
//! its Thing's actions call it on its device's thread, its errors say what
//! to do, `reconnect` makes it again after a failure or a panic, and the
//! server's stop closes it.

use std::io;
use std::time::Duration;

use teta_wot::prelude::*;
use teta_wot::testing::Harness;

use microscope_things::drivers::device_error;
use microscope_things::drivers::serial::{Line, Script};

/// A driver that asks its device questions, and says goodbye on closing.
#[derive(Default)]
struct Asker(Script);

impl Driver for Asker {
    fn close(&mut self) -> anyhow::Result<()> {
        Ok(self.0.send("bye")?)
    }
}

#[derive(Thing)]
struct AskingThing {
    #[device]
    device: Device<Asker>,
}

#[thing_impl]
impl AskingThing {
    /// Asks the device `question`.
    #[action]
    async fn ask(&self, question: String) -> Result<String, ActionError> {
        let ask = move |asker: &mut Asker, _: &CancelToken| -> io::Result<String> {
            asker.0.send(&question)?;
            asker.0.receive(Duration::from_millis(100))
        };
        self.device.try_call(ask).await.map_err(device_error)
    }

    /// A driver's bug.
    #[action]
    async fn crash(&self) -> Result<(), ActionError> {
        let crash = |_: &mut Asker, _: &CancelToken| panic!("a driver's bug");
        self.device.call(crash).await.map_err(device_error)
    }

    /// Closes the driver, and makes it again.
    #[action]
    async fn reconnect(&self) -> Result<(), ActionError> {
        self.device.reset().await.map_err(device_error)
    }
}

#[tokio::test]
async fn a_driver_reconnects_after_a_failure_or_a_bug_and_closes_with_the_server() {
    let script = Script::new()
        .expect("hello", &["hi"])
        .fail(io::ErrorKind::BrokenPipe)
        // A reset closes the driver first.
        .expect("bye", &[])
        .expect("hello", &["hi again"])
        .expect("bye", &[])
        .expect("hello", &["hi a third time"])
        .expect("bye", &[]);
    let device = Device::new({
        let script = script.clone();
        move || Ok::<_, anyhow::Error>(Asker(script.clone()))
    });
    let harness = Harness::builder("asker", AskingThing { device })
        .start()
        .await
        .expect("started");
    let asker = harness.thing();
    let ask = || asker.ask("hello".to_owned());
    assert_eq!(ask().await.expect("an answer"), "hi");

    // Unplugged: the call fails, and the device works again once reconnected.
    let gone = ask().await.expect_err("gone").to_string();
    assert!(gone.contains("the device has gone"), "{gone}");
    asker.reconnect().await.expect("reconnected");
    assert_eq!(ask().await.expect("an answer"), "hi again");

    // A bug faults the device, until it's reconnected.
    let crashed = asker.crash().await.expect_err("crashed").to_string();
    assert!(crashed.contains("asker stopped working"), "{crashed}");
    let faulted = ask().await.expect_err("faulted").to_string();
    assert_eq!(
        faulted,
        "asker stopped working (a driver's bug): reconnect it"
    );
    asker.reconnect().await.expect("reconnected");
    assert_eq!(ask().await.expect("an answer"), "hi a third time");

    harness.stop().await;
    assert_eq!(script.remaining(), 0, "{:?}", script.sent());
}
