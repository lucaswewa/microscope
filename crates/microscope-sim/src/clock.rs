//! Time, injected (ADR-0005): the simulator never reads the system clock.
//! Its models take the time as an argument; a [`Clock`] says what time it is.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Says what time it is: the time since some fixed start.
pub trait Clock: Send + Sync {
    /// The time now.
    fn now(&self) -> Duration;
}

/// A clock that moves only when told, for tests and examples.
#[derive(Debug, Default)]
pub struct ManualClock {
    nanos: AtomicU64,
}

impl ManualClock {
    /// A clock at zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves the clock forward by `by`.
    pub fn advance(&self, by: Duration) {
        let nanos = u64::try_from(by.as_nanos()).unwrap_or(u64::MAX);
        self.nanos.fetch_add(nanos, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.nanos.load(Ordering::SeqCst))
    }
}
