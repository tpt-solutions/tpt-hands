// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Watchdog and safe-state logic.
//!
//! The firmware feeds the watchdog from its control loop. If the loop stops, or
//! the host stops answering and the firmware is waiting on it, the watchdog trips
//! and the outputs go to the safe state.
//!
//! The watchdog latches. Once it has tripped, only an explicit `reset` clears it.
//! A feed that arrives late does not. A loop that wakes up after a stall must not
//! quietly resume driving the hardware.
//!
//! Like the rest of this crate, it is plain arithmetic with no clock of its own.
//! The caller supplies `now_us`.

/// Outputs the firmware drives. The safe state is the value of
/// [`Outputs::SAFE`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outputs {
    /// Whether the power relay is closed. Open is the safe state.
    pub relay_closed: bool,
    /// Grip force commanded, in newtons. Zero is the safe state.
    pub grip_force_n: f32,
}

impl Outputs {
    /// Relay open, grip released.
    pub const SAFE: Outputs = Outputs {
        relay_closed: false,
        grip_force_n: 0.0,
    };
}

/// Whether the control loop is still alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    /// The loop has fed the watchdog within the timeout.
    Alive,
    /// The watchdog has tripped. Drive the outputs to [`Outputs::SAFE`].
    Tripped,
}

/// A latching watchdog.
///
/// ```
/// use tpt_hands_edge::safety::{Health, Watchdog};
///
/// let mut wd = Watchdog::new(10_000, 0);
/// wd.feed(5_000);
/// assert_eq!(wd.check(12_000), Health::Alive);
/// assert_eq!(wd.check(20_000), Health::Tripped);
/// // A late feed does not clear a trip.
/// wd.feed(20_001);
/// assert_eq!(wd.check(20_002), Health::Tripped);
/// wd.reset(20_002);
/// assert_eq!(wd.check(20_003), Health::Alive);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Watchdog {
    timeout_us: u64,
    last_feed_us: u64,
    tripped: bool,
}

impl Watchdog {
    /// Creates an armed watchdog. The first deadline runs from `now_us`.
    pub fn new(timeout_us: u64, now_us: u64) -> Self {
        Self {
            timeout_us,
            last_feed_us: now_us,
            tripped: false,
        }
    }

    /// Records a feed from the control loop. Ignored once the watchdog has
    /// tripped.
    pub fn feed(&mut self, now_us: u64) {
        if !self.tripped {
            self.last_feed_us = now_us;
        }
    }

    /// Returns the health at `now_us`, and trips the watchdog if its deadline has
    /// passed. Once tripped, it stays tripped until [`Watchdog::reset`].
    pub fn check(&mut self, now_us: u64) -> Health {
        if !self.tripped && now_us.saturating_sub(self.last_feed_us) > self.timeout_us {
            self.tripped = true;
        }
        if self.tripped {
            Health::Tripped
        } else {
            Health::Alive
        }
    }

    /// Clears a trip and re-arms the watchdog from `now_us`. The firmware calls
    /// this only after the cause has been dealt with, and it must not be called in
    /// a loop to hide a trip.
    pub fn reset(&mut self, now_us: u64) {
        self.tripped = false;
        self.last_feed_us = now_us;
    }

    /// Whether the watchdog has tripped and not been reset.
    pub fn is_tripped(&self) -> bool {
        self.tripped
    }
}

/// The outputs to drive at this moment. The safe state while the watchdog is
/// tripped, and `wanted` otherwise.
pub fn outputs_for(health: Health, wanted: Outputs) -> Outputs {
    match health {
        Health::Alive => wanted,
        Health::Tripped => Outputs::SAFE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ON: Outputs = Outputs {
        relay_closed: true,
        grip_force_n: 12.0,
    };

    #[test]
    fn stays_alive_while_fed() {
        let mut wd = Watchdog::new(1_000, 0);
        for t in (0..10_000).step_by(500) {
            wd.feed(t);
            assert_eq!(wd.check(t), Health::Alive);
        }
    }

    #[test]
    fn trips_at_the_first_check_after_the_deadline() {
        let mut wd = Watchdog::new(1_000, 0);
        assert_eq!(
            wd.check(1_000),
            Health::Alive,
            "deadline itself is not yet late"
        );
        assert_eq!(wd.check(1_001), Health::Tripped);
    }

    #[test]
    fn a_late_feed_does_not_clear_a_trip() {
        let mut wd = Watchdog::new(1_000, 0);
        wd.check(5_000);
        wd.feed(5_001);
        assert_eq!(wd.check(5_002), Health::Tripped);
        assert!(wd.is_tripped());
    }

    #[test]
    fn reset_rearms_from_the_reset_time() {
        let mut wd = Watchdog::new(1_000, 0);
        wd.check(5_000);
        wd.reset(5_000);
        assert_eq!(wd.check(5_900), Health::Alive);
        assert_eq!(wd.check(6_001), Health::Tripped);
    }

    #[test]
    fn tripped_outputs_are_the_safe_state() {
        assert_eq!(outputs_for(Health::Alive, ON), ON);
        assert_eq!(outputs_for(Health::Tripped, ON), Outputs::SAFE);
        // The safe state is pinned here, so a change to it is a visible decision.
        assert_eq!(
            Outputs::SAFE,
            Outputs {
                relay_closed: false,
                grip_force_n: 0.0
            }
        );
    }

    #[test]
    fn clock_going_backwards_does_not_trip() {
        // saturating_sub keeps a backwards clock from underflowing into a trip.
        let mut wd = Watchdog::new(1_000, 10_000);
        assert_eq!(wd.check(5_000), Health::Alive);
    }
}
