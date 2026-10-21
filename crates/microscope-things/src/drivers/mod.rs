//! What every real driver shares (ADR-0041).
//!
//! - **Its handle,** a serial port or an SDK's camera, lives in a `teta-wot`
//!   [`Device`](teta_wot::Device) on the driver's Thing, made from the
//!   Thing's configuration: `#[device(init = Driver::open(&config.port))]`.
//!   - The device opens before the Thing starts, on a thread of its own,
//!     and runs the driver's calls there, one at a time.
//!   - A device that can't open stops the server from starting, with its
//!     error, as a Thing that can't start does.
//!   - When the server stops, the device closes the driver.
//! - **Its errors** become an action's through [`device_error`], which says
//!   what to do.
//! - **Reconnecting:** each driver's Thing has a `reconnect` action, which
//!   resets its device: the driver is closed, and made again from the
//!   configuration. A call that panics faults the device until then.
//! - **A configuration** names a stage by its serial port (`port`) and a
//!   camera by its serial number (`serial_number`).
//! - **Hardware tests** run only where [`hardware_device`] finds their
//!   device named in the environment.

use teta_wot::{ActionError, DeviceError};

pub mod serial;

/// An action's error for a device's: what went wrong, and what to do.
pub fn device_error(error: DeviceError) -> ActionError {
    // A device's name is `<thing>:<field>`.
    let thing = |name: &str| name.split(':').next().unwrap_or_default().to_owned();
    ActionError::handled(match error {
        DeviceError::NotOpen(name) => format!("{} isn't connected: reconnect it", thing(&name)),
        DeviceError::Faulted { name, reason }
        | DeviceError::Panicked {
            name,
            message: reason,
        } => {
            format!("{} stopped working ({reason}): reconnect it", thing(&name))
        }
        DeviceError::Timeout { name, timeout } => {
            format!("{} didn't answer within {timeout:?}", thing(&name))
        }
        DeviceError::Driver(error) => format!("{error:#}"),
        other => other.to_string(),
    })
}

/// The device a hardware test runs on, which `variable` names, such as
/// `MICROSCOPE_ZABER_PORT=COM3`. Without it, the test is skipped, and says
/// so:
///
/// ```no_run
/// # use microscope_things::drivers::hardware_device;
/// let Some(port) = hardware_device("MICROSCOPE_ZABER_PORT") else {
///     return;
/// };
/// ```
pub fn hardware_device(variable: &str) -> Option<String> {
    let device = std::env::var(variable)
        .ok()
        .filter(|value| !value.is_empty());
    if device.is_none() {
        eprintln!("skipped: set {variable} to run this test on its device");
    }
    device
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn device_errors_say_what_to_do() {
        let message = |error| device_error(error).to_string();
        let name = || "stage:device".to_owned();
        assert_eq!(
            message(DeviceError::NotOpen(name())),
            "stage isn't connected: reconnect it"
        );
        let faulted = DeviceError::Faulted {
            name: name(),
            reason: "a bug".to_owned(),
        };
        assert_eq!(
            message(faulted),
            "stage stopped working (a bug): reconnect it"
        );
        let timeout = Duration::from_secs(2);
        assert_eq!(
            message(DeviceError::Timeout {
                name: name(),
                timeout
            }),
            "stage didn't answer within 2s"
        );
        let driver = DeviceError::Driver(anyhow::anyhow!("no answer").context("homing"));
        assert_eq!(message(driver), "homing: no answer");
    }

    #[test]
    fn a_hardware_test_runs_only_where_its_device_is_named() {
        assert_eq!(hardware_device("MICROSCOPE_NO_SUCH_DEVICE"), None);
        // Cargo names the package's folder for its tests.
        assert!(hardware_device("CARGO_MANIFEST_DIR").is_some());
    }
}
