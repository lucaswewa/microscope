//! When the launcher starts the server again (ADR-0033): at once after it
//! asks to restart, after a growing delay after it crashes, and not at all
//! after it stops, can't start, or crashes too often.

use std::time::{Duration, Instant};

/// The delay after a first crash; each crash close after it doubles it.
pub const FIRST_DELAY: Duration = Duration::from_secs(1);
/// How long a crash counts towards a crash loop.
pub const WINDOW: Duration = Duration::from_secs(60);
/// How many crashes within [`WINDOW`] the launcher tolerates.
pub const CRASH_LIMIT: usize = 5;

/// What to do once the server has exited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// Start it again after this long.
    Restart(Duration),
    /// Stop the launcher, with this exit code.
    Stop(u8),
}

/// The server's recent crashes, which decide what to do next.
#[derive(Debug, Default)]
pub struct Policy {
    crashes: Vec<Instant>,
}

impl Policy {
    /// What to do after the server exited with `code` at `now`. `None` is
    /// an exit without a code, or one the launcher forced.
    pub fn after_exit(&mut self, code: Option<i32>, now: Instant) -> Next {
        match code {
            // Stopped, as asked.
            Some(0) => Next::Stop(0),
            // A restart, asked for through `system.restart` (ADR-0032).
            Some(75) => Next::Restart(Duration::ZERO),
            // Its command line or configuration is wrong: starting it again
            // won't help.
            Some(code @ (2 | 3)) => Next::Stop(code as u8),
            _ => {
                self.crashes
                    .retain(|&crash| now.duration_since(crash) < WINDOW);
                self.crashes.push(now);
                let crashes = self.crashes.len();
                if crashes > CRASH_LIMIT {
                    return Next::Stop(1);
                }
                Next::Restart(FIRST_DELAY * (1 << (crashes - 1)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn a_stop_ends_the_launcher_and_a_restart_is_at_once() {
        let mut policy = Policy::default();
        let now = Instant::now();
        assert_eq!(policy.after_exit(Some(0), now), Next::Stop(0));
        for _ in 0..10 {
            assert_eq!(
                policy.after_exit(Some(75), now),
                Next::Restart(Duration::ZERO)
            );
        }
        // Restarts aren't crashes.
        assert_eq!(policy.after_exit(Some(1), now), Next::Restart(SECOND));
    }

    #[test]
    fn a_server_that_cant_start_isnt_started_again() {
        let mut policy = Policy::default();
        let now = Instant::now();
        assert_eq!(policy.after_exit(Some(2), now), Next::Stop(2));
        assert_eq!(policy.after_exit(Some(3), now), Next::Stop(3));
    }

    #[test]
    fn crashes_back_off_until_too_many_come_close_together() {
        let mut policy = Policy::default();
        let start = Instant::now();
        let delays: Vec<Next> = (0..6)
            .map(|i| policy.after_exit(if i % 2 == 0 { Some(1) } else { None }, start + i * SECOND))
            .collect();
        let restarts = [1, 2, 4, 8, 16].map(|s| Next::Restart(s * SECOND));
        assert_eq!(delays[..5], restarts);
        assert_eq!(delays[5], Next::Stop(1));
    }

    #[test]
    fn crashes_a_minute_apart_dont_add_up() {
        let mut policy = Policy::default();
        let start = Instant::now();
        for i in 0..10 {
            let at = start + i * (WINDOW + SECOND);
            assert_eq!(policy.after_exit(Some(1), at), Next::Restart(SECOND));
        }
    }
}
