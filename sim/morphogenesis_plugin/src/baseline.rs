// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! A scripted baseline agent. It needs no tpt-cerebrum.
//!
//! The script is a fixed sequence, so the result is the same on every run:
//!
//! 1. grasp the object with the healthy finger
//! 2. tear the tendon, then wait for the healing timer
//! 3. reinforce the tendon with hydrogel
//! 4. grasp the object again
//!
//! A learned agent can be compared against this run. If it does worse, it has
//! not learned to use the healing and reinforcement actions.

use crate::{Contact, Finger, ModelError, run_grasp};

/// Parameters of one scripted episode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Episode {
    /// Grip force on the object, in newtons.
    pub grip_force_n: f64,
    /// Contact force that counts as a grasp, in newtons.
    pub threshold_n: f64,
    /// Time the threshold must hold, in seconds.
    pub hold_s: f64,
    /// Hydrogel volume used to reinforce, in millilitres.
    pub reinforce_ml: f64,
    /// Simulation step, in seconds.
    pub dt: f64,
}

impl Default for Episode {
    fn default() -> Self {
        Self {
            grip_force_n: 2.0,
            threshold_n: 0.1,
            hold_s: 0.2,
            reinforce_ml: 1.0,
            dt: 1.0e-3,
        }
    }
}

/// What the baseline agent observed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Report {
    /// Did the grasp before the injury succeed?
    pub first_grasp_ok: bool,
    /// Simulated seconds spent waiting for the tendon to heal.
    pub healing_wait_s: f64,
    /// Did the grasp after healing and reinforcement succeed?
    pub second_grasp_ok: bool,
    /// Joint stiffness after reinforcement, in Nm/rad.
    pub final_stiffness_nm_per_rad: f64,
}

/// Runs the scripted sequence on `finger` against `object`.
///
/// The healing wait is bounded by the healing time plus a margin, so a
/// finger that never heals cannot hang the run.
pub fn run_scripted(
    finger: &mut Finger,
    object: Contact,
    episode: Episode,
) -> Result<Report, ModelError> {
    let grasp = |f: &mut Finger| {
        run_grasp(
            f,
            object,
            episode.grip_force_n,
            episode.threshold_n,
            episode.hold_s,
            10.0,
            episode.dt,
        )
    };

    let first = grasp(finger)?;

    finger.injure();
    let healing_limit_s = finger.healing_time_s() * 2.0 + 1.0;
    let start = finger.time_s();
    while finger.is_healing() && finger.time_s() - start < healing_limit_s {
        finger.step(episode.dt);
    }
    let healing_wait_s = finger.time_s() - start;

    finger.reinforce(episode.reinforce_ml)?;
    let second = grasp(finger)?;

    Ok(Report {
        first_grasp_ok: first.success,
        healing_wait_s,
        second_grasp_ok: second.success,
        final_stiffness_nm_per_rad: finger.stiffness_nm_per_rad(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FingerParams;

    fn object() -> Contact {
        Contact {
            angle_rad: 0.2,
            stiffness_nm_per_rad: 0.5,
        }
    }

    #[test]
    fn baseline_grasps_before_and_after_repair() {
        let mut finger = Finger::new(FingerParams::default());
        let report = run_scripted(&mut finger, object(), Episode::default()).unwrap();
        assert!(report.first_grasp_ok);
        assert!(report.second_grasp_ok);
    }

    #[test]
    fn healing_wait_is_about_the_healing_time() {
        let p = FingerParams::default();
        let mut finger = Finger::new(p);
        let report = run_scripted(&mut finger, object(), Episode::default()).unwrap();
        assert!(
            (report.healing_wait_s - p.healing_time_s).abs() < 1.0e-2,
            "waited {} s, healing time {} s",
            report.healing_wait_s,
            p.healing_time_s
        );
    }

    #[test]
    fn reinforcement_raises_stiffness() {
        let p = FingerParams::default();
        let mut finger = Finger::new(p);
        let report = run_scripted(&mut finger, object(), Episode::default()).unwrap();
        let expected = p.stiffness_nm_per_rad + p.reinforce_gain_nm_per_rad_per_ml * 1.0;
        assert!((report.final_stiffness_nm_per_rad - expected).abs() < 1.0e-9);
    }

    #[test]
    fn negative_reinforcement_is_rejected() {
        let mut finger = Finger::new(FingerParams::default());
        let episode = Episode {
            reinforce_ml: -1.0,
            ..Episode::default()
        };
        assert!(run_scripted(&mut finger, object(), episode).is_err());
    }
}
