//! Periods are simple measurements of time with regards to minting
//!
//! Specifically, a period is a duration with a specific length. The duration used is calculated in
//! the v1 minting, such that there were exactly 60 periods in the 5 years a node would receive
//! tokens.

use serde::{Deserialize, Serialize};

/// Timestamp of the start of the first period.
const FIRST_PERIOD_START_TIMESTAMP: i64 = 1522501000;
/// The duration of a standard period, as used by the minting payouts, in seconds.
const STANDARD_PERIOD_DURATION: u64 = 24 * 60 * 60 * (365 * 3 + 366 * 2) / 60;

/// A period represents a timestamp used by the minting process.
///
/// Periods are defined such that there are roughly 12 periods per year.
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Period {
    start: i64,
    end: i64,
}

impl Period {
    /// Get the current payment cycle.
    pub fn current() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let offset = (now - FIRST_PERIOD_START_TIMESTAMP as u64) / STANDARD_PERIOD_DURATION;
        let start = FIRST_PERIOD_START_TIMESTAMP + (STANDARD_PERIOD_DURATION * offset) as i64;
        Period {
            start,
            end: start + STANDARD_PERIOD_DURATION as i64,
        }
    }

    /// Get the period with the given offset from the start.
    pub fn at_offset(offset: i64) -> Self {
        Period {
            start: FIRST_PERIOD_START_TIMESTAMP + STANDARD_PERIOD_DURATION as i64 * offset,
            end: FIRST_PERIOD_START_TIMESTAMP + STANDARD_PERIOD_DURATION as i64 * (offset + 1),
        }
    }

    /// Start timestamp of the period.
    pub fn start(&self) -> i64 {
        self.start
    }

    /// End timestamp of the period.
    pub fn end(&self) -> i64 {
        self.end
    }

    /// The duration of the period in seconds.
    pub fn duration(&self) -> u64 {
        (self.end - self.start) as u64
    }

    /// Indicates if a given timestamp is part of the period or not.
    pub fn timestamp_in_period(&self, ts: i64) -> bool {
        ts >= self.start && ts <= self.end
    }

    /// Adjusts the start time of this period.
    ///
    /// # Panics
    ///
    /// This function will panic if the provided new start time is bigger than the already set end
    /// time of the period.
    pub fn scale_start(&mut self, ts: i64) {
        assert!(ts <= self.end);
        self.start = ts;
    }
}
