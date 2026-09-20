//! The injected time source.
//!
//! A leaf module with no dependencies of its own, so `policy` can stay clock-free:
//! `policy::evaluate` is handed one `now`, it never reads one.

use std::time::Instant;

/// Every time reading in the application goes through this trait, so tests can
/// control both the wall clock and the monotonic clock.
pub trait Clock: Send + Sync + 'static {
    /// UTC microseconds since the Unix epoch.
    fn now_utc_micros(&self) -> i64;

    /// A monotonic reading, used for deadlines that must survive a wall-clock jump.
    fn now_monotonic(&self) -> Instant;
}

/// The production clock. Named only in `main.rs` and in the test harness.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc_micros(&self) -> i64 {
        jiff::Timestamp::now().as_microsecond()
    }

    fn now_monotonic(&self) -> Instant {
        Instant::now()
    }
}
