// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Local reflexes that run on the edge, without waiting for the host.
//!
//! - `PathwayMonitor`: detects a tear from a resistance spike, debounced, and
//!   detects healing when resistance falls back.
//! - `pick_route`: fail-soft. Telemetry moves to the first healthy pathway in
//!   the preference order.
//! - `GripReflex`: overload release, and slip detection from a sudden force drop.
//!
//! All of it is plain arithmetic with no heap and no clock. The caller feeds in
//! samples from the 1 kHz loop, so the logic can be tested on the host.

/// Health of one pathway, as the edge sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathwayStatus {
    /// Working normally.
    Intact,
    /// Reinforced by a host command (tendon thickening). Still usable, and
    /// still watched for tears.
    Hypertrophied,
    /// Torn. Telemetry is rerouted away from it.
    Severed,
}

/// Something the monitor noticed on this sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathwayEvent {
    /// Nothing changed.
    None,
    /// The pathway was torn. Report it to tpt-soma.
    Severed,
    /// Resistance has come back down, so the pathway is healed.
    Healed,
}

/// Watches one pathway's resistance and reports tears and healing.
///
/// A change of state needs `debounce` samples in a row. One noisy sample does
/// not cause a tear.
///
/// ```
/// use tpt_hands_edge::reflex::{PathwayEvent, PathwayMonitor};
///
/// let mut m = PathwayMonitor::new(100.0, 50.0, 3);
/// assert_eq!(m.sample(500.0), PathwayEvent::None);
/// assert_eq!(m.sample(500.0), PathwayEvent::None);
/// assert_eq!(m.sample(500.0), PathwayEvent::Severed);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathwayMonitor {
    status: PathwayStatus,
    sever_ohms: f32,
    recover_ohms: f32,
    debounce: u8,
    streak: u8,
}

impl PathwayMonitor {
    /// Creates a monitor, starting `Intact`.
    ///
    /// `sever_ohms` is the resistance at or above which a sample counts as a
    /// tear. `recover_ohms` is the resistance at or below which a severed
    /// pathway counts as healed. `recover_ohms` must be below `sever_ohms`, so
    /// that the two thresholds leave a gap and the state does not chatter.
    pub fn new(sever_ohms: f32, recover_ohms: f32, debounce: u8) -> Self {
        Self {
            status: PathwayStatus::Intact,
            sever_ohms,
            recover_ohms,
            debounce: debounce.max(1),
            streak: 0,
        }
    }

    /// Current status.
    pub fn status(&self) -> PathwayStatus {
        self.status
    }

    /// Marks a usable pathway as reinforced, on the host's command after a
    /// `reinforce` morphogenesis ack. A severed pathway is not changed, because
    /// it must heal first.
    ///
    /// ```
    /// use tpt_hands_edge::reflex::{PathwayMonitor, PathwayStatus};
    ///
    /// let mut m = PathwayMonitor::new(100.0, 50.0, 1);
    /// m.reinforce();
    /// assert_eq!(m.status(), PathwayStatus::Hypertrophied);
    /// ```
    pub fn reinforce(&mut self) {
        if self.status == PathwayStatus::Intact {
            self.status = PathwayStatus::Hypertrophied;
            self.streak = 0;
        }
    }

    /// Feeds one resistance sample and returns any change it caused.
    ///
    /// A NaN or infinite reading counts as a tear. Failing safe is better than
    /// assuming the pathway is fine.
    pub fn sample(&mut self, ohms: f32) -> PathwayEvent {
        match self.status {
            PathwayStatus::Intact | PathwayStatus::Hypertrophied => {
                if !ohms.is_finite() || ohms >= self.sever_ohms {
                    self.streak = self.streak.saturating_add(1);
                    if self.streak >= self.debounce {
                        self.status = PathwayStatus::Severed;
                        self.streak = 0;
                        return PathwayEvent::Severed;
                    }
                } else {
                    self.streak = 0;
                }
            }
            PathwayStatus::Severed => {
                if ohms.is_finite() && ohms <= self.recover_ohms {
                    self.streak = self.streak.saturating_add(1);
                    if self.streak >= self.debounce {
                        self.status = PathwayStatus::Intact;
                        self.streak = 0;
                        return PathwayEvent::Healed;
                    }
                } else {
                    self.streak = 0;
                }
            }
        }
        PathwayEvent::None
    }
}

/// Picks the first intact pathway in `preference` order, or `None` if every
/// preferred pathway is severed.
///
/// `monitors[i]` is the monitor for pathway `i`. Entries of `preference` that
/// are out of range are skipped.
///
/// ```
/// use tpt_hands_edge::reflex::{pick_route, PathwayMonitor};
///
/// let mut monitors = [PathwayMonitor::new(100.0, 50.0, 1); 2];
/// monitors[0].sample(500.0); // pathway 0 is torn
/// assert_eq!(pick_route(&monitors, &[0, 1]), Some(1));
/// ```
pub fn pick_route(monitors: &[PathwayMonitor], preference: &[usize]) -> Option<usize> {
    preference.iter().copied().find(|&i| {
        monitors.get(i).is_some_and(|m| {
            matches!(
                m.status(),
                PathwayStatus::Intact | PathwayStatus::Hypertrophied
            )
        })
    })
}

/// What the grip reflex asks the controller to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GripAction {
    /// Nothing to do.
    None,
    /// Force is over the limit. Release now, before the hardware is damaged.
    ReleaseOverload,
    /// Force dropped suddenly, which means the object is slipping. Tighten the
    /// grip by `by_n` newtons, without going over the limit.
    Tighten {
        /// Extra force to apply, in newtons.
        by_n: f32,
    },
}

/// Slip and overload reflexes for the grip.
///
/// ```
/// use tpt_hands_edge::reflex::{GripAction, GripReflex};
///
/// let mut grip = GripReflex::new(30.0, 5.0, 2.0);
/// assert_eq!(grip.on_force(20.0), GripAction::None);
/// assert_eq!(grip.on_force(12.0), GripAction::Tighten { by_n: 2.0 });
/// assert_eq!(grip.on_force(31.0), GripAction::ReleaseOverload);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GripReflex {
    max_force_n: f32,
    slip_drop_n: f32,
    tighten_n: f32,
    last_force_n: Option<f32>,
}

impl GripReflex {
    /// `max_force_n` is the overload limit. `slip_drop_n` is the drop between
    /// samples that counts as slipping. `tighten_n` is the extra force applied
    /// on a slip.
    pub fn new(max_force_n: f32, slip_drop_n: f32, tighten_n: f32) -> Self {
        Self {
            max_force_n,
            slip_drop_n,
            tighten_n,
            last_force_n: None,
        }
    }

    /// Feeds one force sample and returns the reflex's action.
    pub fn on_force(&mut self, force_n: f32) -> GripAction {
        if !force_n.is_finite() {
            return GripAction::ReleaseOverload;
        }
        let action = if force_n > self.max_force_n {
            GripAction::ReleaseOverload
        } else if let Some(last) = self.last_force_n {
            if last - force_n >= self.slip_drop_n {
                GripAction::Tighten {
                    by_n: self.tighten_n,
                }
            } else {
                GripAction::None
            }
        } else {
            GripAction::None
        };
        self.last_force_n = Some(force_n);
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_noisy_sample_does_not_tear_a_pathway() {
        let mut m = PathwayMonitor::new(100.0, 50.0, 3);
        assert_eq!(m.sample(500.0), PathwayEvent::None);
        assert_eq!(m.sample(10.0), PathwayEvent::None);
        assert_eq!(m.sample(500.0), PathwayEvent::None);
        assert_eq!(m.status(), PathwayStatus::Intact);
    }

    #[test]
    fn sustained_spike_severs_then_sustained_low_heals() {
        let mut m = PathwayMonitor::new(100.0, 50.0, 3);
        let mut last = PathwayEvent::None;
        for _ in 0..3 {
            last = m.sample(500.0);
        }
        assert_eq!(last, PathwayEvent::Severed);
        assert_eq!(m.status(), PathwayStatus::Severed);

        // A reading in the gap between the thresholds does not heal it.
        for _ in 0..10 {
            assert_eq!(m.sample(75.0), PathwayEvent::None);
        }
        assert_eq!(m.status(), PathwayStatus::Severed);

        for _ in 0..3 {
            last = m.sample(10.0);
        }
        assert_eq!(last, PathwayEvent::Healed);
        assert_eq!(m.status(), PathwayStatus::Intact);
    }

    #[test]
    fn reinforced_pathway_is_still_watched_for_tears() {
        let mut m = PathwayMonitor::new(100.0, 50.0, 2);
        m.reinforce();
        assert_eq!(m.status(), PathwayStatus::Hypertrophied);

        assert_eq!(m.sample(500.0), PathwayEvent::None);
        assert_eq!(m.sample(500.0), PathwayEvent::Severed);
        assert_eq!(m.status(), PathwayStatus::Severed);
    }

    #[test]
    fn reinforce_does_not_change_a_severed_pathway() {
        let mut m = PathwayMonitor::new(100.0, 50.0, 1);
        m.sample(500.0);
        m.reinforce();
        assert_eq!(m.status(), PathwayStatus::Severed);
    }

    #[test]
    fn reinforced_pathway_still_routes_and_heals_to_intact() {
        let mut monitors = [PathwayMonitor::new(100.0, 50.0, 1); 2];
        monitors[0].reinforce();
        assert_eq!(pick_route(&monitors, &[0, 1]), Some(0));

        monitors[0].sample(500.0);
        monitors[0].sample(10.0);
        assert_eq!(monitors[0].status(), PathwayStatus::Intact);
    }

    #[test]
    fn non_finite_resistance_fails_safe() {
        let mut m = PathwayMonitor::new(100.0, 50.0, 1);
        assert_eq!(m.sample(f32::NAN), PathwayEvent::Severed);
    }

    #[test]
    fn route_falls_back_to_next_intact_pathway() {
        let mut monitors = [PathwayMonitor::new(100.0, 50.0, 1); 3];
        assert_eq!(pick_route(&monitors, &[0, 1, 2]), Some(0));

        monitors[0].sample(500.0);
        assert_eq!(pick_route(&monitors, &[0, 1, 2]), Some(1));

        monitors[1].sample(500.0);
        monitors[2].sample(500.0);
        assert_eq!(pick_route(&monitors, &[0, 1, 2]), None);
    }

    #[test]
    fn route_ignores_out_of_range_preferences() {
        let monitors = [PathwayMonitor::new(100.0, 50.0, 1)];
        assert_eq!(pick_route(&monitors, &[9, 0]), Some(0));
    }

    #[test]
    fn overload_releases_immediately() {
        let mut grip = GripReflex::new(30.0, 5.0, 2.0);
        assert_eq!(grip.on_force(30.0), GripAction::None);
        assert_eq!(grip.on_force(30.5), GripAction::ReleaseOverload);
    }

    #[test]
    fn slip_needs_a_drop_not_a_steady_grip() {
        let mut grip = GripReflex::new(30.0, 5.0, 2.0);
        assert_eq!(grip.on_force(20.0), GripAction::None);
        assert_eq!(grip.on_force(18.0), GripAction::None); // drop of 2, under the limit
        assert_eq!(grip.on_force(12.0), GripAction::Tighten { by_n: 2.0 });
    }

    #[test]
    fn non_finite_force_releases() {
        let mut grip = GripReflex::new(30.0, 5.0, 2.0);
        assert_eq!(grip.on_force(f32::INFINITY), GripAction::ReleaseOverload);
    }
}
