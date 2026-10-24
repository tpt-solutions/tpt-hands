// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! In-house simulation of a tendon-driven finger (DESIGN.md Phase 4).
//!
//! This is a deliberately small model. It has one joint, a tendon that pulls it
//! closed, a torsional spring that opens it, and damping. It can be torn, healed
//! over simulated time, and reinforced, and it can close on an object. That is
//! enough to exercise the morphogenesis loop end to end.
//!
//! Scope, and what it is not:
//! - One planar joint. Not a hand. Multi-joint models come later.
//! - A penalty contact (a spring that only pushes back). No friction, no slip.
//! - Semi-implicit Euler at a fixed step. Accurate enough for the steady-state
//!   checks below, not for dynamic fidelity.
//! - Morphogenesis effects follow `MorphogenesisAck`. A healed pathway restores
//!   tendon force, and a reinforcement adds stiffness. The constants are
//!   placeholders until they are measured on hardware.
//!
//! Units: radians, newtons, metres, kilograms, seconds.
//!
//! The validation tests check the simulator against closed-form equilibria.

/// Physical constants of the finger. Placeholders, not measured values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FingerParams {
    /// Torsional spring stiffness that opens the joint, in Nm/rad.
    pub stiffness_nm_per_rad: f64,
    /// Rotational inertia, in kg m^2.
    pub inertia_kg_m2: f64,
    /// Viscous damping, in Nm s/rad.
    pub damping_nms_per_rad: f64,
    /// Tendon moment arm, in metres.
    pub moment_arm_m: f64,
    /// Time for a severed tendon to heal, in seconds.
    pub healing_time_s: f64,
    /// Stiffness added per millilitre of reinforcing hydrogel, in Nm/rad per mL.
    pub reinforce_gain_nm_per_rad_per_ml: f64,
}

impl Default for FingerParams {
    fn default() -> Self {
        Self {
            stiffness_nm_per_rad: 0.05,
            inertia_kg_m2: 1.0e-4,
            damping_nms_per_rad: 1.0e-3,
            moment_arm_m: 0.01,
            healing_time_s: 2.0,
            reinforce_gain_nm_per_rad_per_ml: 0.02,
        }
    }
}

/// Why the model refused an input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelError {
    /// A value is NaN or infinite.
    NonFinite(&'static str),
    /// A value is below zero where that makes no physical sense.
    Negative(&'static str),
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFinite(n) => write!(f, "`{n}` is not finite"),
            Self::Negative(n) => write!(f, "`{n}` is negative"),
        }
    }
}

impl std::error::Error for ModelError {}

fn check_non_negative(name: &'static str, value: f64) -> Result<f64, ModelError> {
    if !value.is_finite() {
        Err(ModelError::NonFinite(name))
    } else if value < 0.0 {
        Err(ModelError::Negative(name))
    } else {
        Ok(value)
    }
}

mod hand;
pub use hand::SimHand;

/// A rigid object the finger can close on, modelled as a stop at `angle_rad`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// Joint angle at which the finger touches the object, in radians.
    pub angle_rad: f64,
    /// Penalty stiffness of the object, in Nm/rad. Larger means a harder object.
    pub stiffness_nm_per_rad: f64,
}

/// A single tendon-driven finger.
#[derive(Debug, Clone, PartialEq)]
pub struct Finger {
    params: FingerParams,
    /// Current stiffness. Starts at the parameter value and grows with reinforcement.
    stiffness_nm_per_rad: f64,
    angle_rad: f64,
    angular_velocity: f64,
    grip_force_n: f64,
    /// 1.0 when the tendon is intact. 0.0 when it is severed and not yet healed.
    integrity: f64,
    /// Seconds since the tear, while healing. `None` when there is nothing to heal.
    healing_elapsed_s: Option<f64>,
    contact: Option<Contact>,
    time_s: f64,
}

impl Finger {
    /// Creates a resting, intact finger with no object in contact.
    pub fn new(params: FingerParams) -> Self {
        Self {
            stiffness_nm_per_rad: params.stiffness_nm_per_rad,
            params,
            angle_rad: 0.0,
            angular_velocity: 0.0,
            grip_force_n: 0.0,
            integrity: 1.0,
            healing_elapsed_s: None,
            contact: None,
            time_s: 0.0,
        }
    }

    /// Sets the tendon pull that closes the finger, in newtons.
    pub fn set_grip_force(&mut self, newtons: f64) -> Result<(), ModelError> {
        self.grip_force_n = check_non_negative("grip_force", newtons)?;
        Ok(())
    }

    /// Tears the tendon. Its integrity drops to zero and healing starts.
    pub fn injure(&mut self) {
        self.integrity = 0.0;
        self.healing_elapsed_s = Some(0.0);
    }

    /// Adds stiffness from a reinforcing hydrogel, as a `ReinforceTendon`
    /// morphogenesis action does.
    pub fn reinforce(&mut self, volume_ml: f64) -> Result<(), ModelError> {
        let v = check_non_negative("volume_ml", volume_ml)?;
        self.stiffness_nm_per_rad += self.params.reinforce_gain_nm_per_rad_per_ml * v;
        Ok(())
    }

    /// Places a rigid object for the finger to close on.
    pub fn place_object(&mut self, contact: Contact) -> Result<(), ModelError> {
        if !contact.angle_rad.is_finite() {
            return Err(ModelError::NonFinite("contact.angle_rad"));
        }
        check_non_negative("contact.stiffness_nm_per_rad", contact.stiffness_nm_per_rad)?;
        self.contact = Some(contact);
        Ok(())
    }

    /// Removes the object.
    pub fn remove_object(&mut self) {
        self.contact = None;
    }

    /// Advances the model by `dt` seconds, including the healing timer.
    pub fn step(&mut self, dt: f64) {
        if dt <= 0.0 || !dt.is_finite() {
            return;
        }
        self.time_s += dt;
        self.advance_healing(dt);

        let theta = self.angle_rad;
        let tendon = self.grip_force_n * self.integrity * self.params.moment_arm_m;
        let spring = -self.stiffness_nm_per_rad * theta;
        let damping = -self.params.damping_nms_per_rad * self.angular_velocity;
        let contact = match self.contact {
            Some(c) if theta > c.angle_rad => -c.stiffness_nm_per_rad * (theta - c.angle_rad),
            _ => 0.0,
        };
        let torque = tendon + spring + damping + contact;

        self.angular_velocity += torque / self.params.inertia_kg_m2 * dt;
        self.angle_rad += self.angular_velocity * dt;
    }

    fn advance_healing(&mut self, dt: f64) {
        if let Some(elapsed) = self.healing_elapsed_s {
            let elapsed = elapsed + dt;
            if elapsed >= self.params.healing_time_s {
                self.integrity = 1.0;
                self.healing_elapsed_s = None;
            } else {
                self.integrity = elapsed / self.params.healing_time_s;
                self.healing_elapsed_s = Some(elapsed);
            }
        }
    }

    /// Grip force currently commanded, in newtons.
    pub fn grip_force_n(&self) -> f64 {
        self.grip_force_n
    }

    /// Joint angle, in radians.
    pub fn angle_rad(&self) -> f64 {
        self.angle_rad
    }

    /// Tendon integrity, from 0.0 (severed) to 1.0 (intact).
    pub fn integrity(&self) -> f64 {
        self.integrity
    }

    /// True while a tear is healing.
    pub fn is_healing(&self) -> bool {
        self.healing_elapsed_s.is_some()
    }

    /// Current joint stiffness, in Nm/rad.
    pub fn stiffness_nm_per_rad(&self) -> f64 {
        self.stiffness_nm_per_rad
    }

    /// Time for a severed tendon to heal, in seconds.
    pub fn healing_time_s(&self) -> f64 {
        self.params.healing_time_s
    }

    /// Simulated time since the finger was created, in seconds.
    pub fn time_s(&self) -> f64 {
        self.time_s
    }

    /// Force the finger puts on the object, in newtons. Zero without contact.
    pub fn contact_force_n(&self) -> f64 {
        match self.contact {
            Some(c) if self.angle_rad > c.angle_rad => {
                c.stiffness_nm_per_rad * (self.angle_rad - c.angle_rad) / self.params.moment_arm_m
            }
            _ => 0.0,
        }
    }
}

/// Result of a grasp task.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraspResult {
    /// True if the contact force reached the threshold and held for the hold time.
    pub success: bool,
    /// Simulated time at which the task finished, in seconds.
    pub finished_at_s: f64,
    /// Contact force at the end of the run, in newtons.
    pub final_force_n: f64,
}

/// Closes the finger on an object with `grip_force_n` and holds until the
/// contact force has been at least `threshold_n` for `hold_s`. Gives up after
/// `max_s` simulated seconds.
pub fn run_grasp(
    finger: &mut Finger,
    object: Contact,
    grip_force_n: f64,
    threshold_n: f64,
    hold_s: f64,
    max_s: f64,
    dt: f64,
) -> Result<GraspResult, ModelError> {
    finger.place_object(object)?;
    finger.set_grip_force(grip_force_n)?;
    let mut held = 0.0;
    while finger.time_s() < max_s {
        finger.step(dt);
        if finger.contact_force_n() >= threshold_n {
            held += dt;
            if held >= hold_s {
                // Read the force before removing the object, or it reads as zero.
                let force = finger.contact_force_n();
                finger.remove_object();
                return Ok(GraspResult {
                    success: true,
                    finished_at_s: finger.time_s(),
                    final_force_n: force,
                });
            }
        } else {
            held = 0.0;
        }
    }
    let force = finger.contact_force_n();
    finger.remove_object();
    Ok(GraspResult {
        success: false,
        finished_at_s: finger.time_s(),
        final_force_n: force,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0e-3;

    /// Runs until the joint has settled. Long enough for the damped response.
    fn settle(finger: &mut Finger, seconds: f64) {
        let steps = (seconds / DT) as usize;
        for _ in 0..steps {
            finger.step(DT);
        }
    }

    #[test]
    fn free_finger_settles_at_the_closed_form_angle() {
        // No contact: spring torque equals tendon torque, so theta = F r i / k.
        let p = FingerParams::default();
        let mut f = Finger::new(p);
        f.set_grip_force(2.0).unwrap();
        settle(&mut f, 20.0);
        let expected = 2.0 * p.moment_arm_m * 1.0 / p.stiffness_nm_per_rad;
        assert!(
            (f.angle_rad() - expected).abs() < 1.0e-3,
            "angle {} vs {}",
            f.angle_rad(),
            expected
        );
    }

    #[test]
    fn released_finger_returns_to_rest() {
        let mut f = Finger::new(FingerParams::default());
        f.set_grip_force(2.0).unwrap();
        settle(&mut f, 20.0);
        f.set_grip_force(0.0).unwrap();
        settle(&mut f, 20.0);
        assert!(f.angle_rad().abs() < 1.0e-3, "angle {}", f.angle_rad());
    }

    #[test]
    fn contact_force_matches_the_closed_form_grasp() {
        // Object at theta_c with stiffness K. Balance:
        // F r = k theta + K (theta - theta_c), so theta = (F r + K theta_c) / (k + K).
        // Force on the object is K (theta - theta_c) / r.
        let p = FingerParams::default();
        let object = Contact {
            angle_rad: 0.2,
            stiffness_nm_per_rad: 0.5,
        };
        let f_grip = 2.0;
        let k = p.stiffness_nm_per_rad;
        let big_k = object.stiffness_nm_per_rad;
        let theta = (f_grip * p.moment_arm_m + big_k * object.angle_rad) / (k + big_k);
        let expected_force = big_k * (theta - object.angle_rad) / p.moment_arm_m;

        let mut f = Finger::new(p);
        f.place_object(object).unwrap();
        f.set_grip_force(f_grip).unwrap();
        settle(&mut f, 20.0);
        assert!(
            (f.contact_force_n() - expected_force).abs() < 1.0e-3,
            "force {} vs {}",
            f.contact_force_n(),
            expected_force
        );
    }

    #[test]
    fn severed_tendon_cannot_close_the_finger() {
        // Checked one step after the tear, before healing has any effect. The
        // tendon's share of the torque is then almost zero, so the joint barely moves.
        let mut f = Finger::new(FingerParams::default());
        f.injure();
        f.set_grip_force(2.0).unwrap();
        f.step(DT);
        assert!(f.angle_rad().abs() < 1.0e-6, "angle {}", f.angle_rad());
    }

    #[test]
    fn healing_runs_on_simulated_time() {
        let mut f = Finger::new(FingerParams::default());
        f.injure();
        assert!(f.is_healing());
        settle(&mut f, 1.0);
        assert!(
            (f.integrity() - 0.5).abs() < 1.0e-2,
            "integrity {}",
            f.integrity()
        );
        settle(&mut f, 1.0 + 1.0e-2);
        assert_eq!(f.integrity(), 1.0);
        assert!(!f.is_healing());
    }

    #[test]
    fn reinforcement_raises_stiffness_and_reduces_closure() {
        let p = FingerParams::default();
        let mut f = Finger::new(p);
        f.reinforce(1.0).unwrap();
        let k2 = p.stiffness_nm_per_rad + p.reinforce_gain_nm_per_rad_per_ml;
        assert!((f.stiffness_nm_per_rad() - k2).abs() < 1.0e-12);

        f.set_grip_force(2.0).unwrap();
        settle(&mut f, 20.0);
        let expected = 2.0 * p.moment_arm_m / k2;
        assert!((f.angle_rad() - expected).abs() < 1.0e-3);
    }

    #[test]
    fn invalid_inputs_are_refused() {
        let mut f = Finger::new(FingerParams::default());
        assert_eq!(
            f.set_grip_force(f64::NAN),
            Err(ModelError::NonFinite("grip_force"))
        );
        assert_eq!(
            f.set_grip_force(-1.0),
            Err(ModelError::Negative("grip_force"))
        );
        assert_eq!(f.reinforce(-0.5), Err(ModelError::Negative("volume_ml")));
    }

    #[test]
    fn grasp_task_succeeds_on_an_intact_finger() {
        let mut f = Finger::new(FingerParams::default());
        let object = Contact {
            angle_rad: 0.2,
            stiffness_nm_per_rad: 0.5,
        };
        let r = run_grasp(&mut f, object, 2.0, 0.5, 0.5, 10.0, DT).unwrap();
        assert!(r.success, "{r:?}");
        assert!(r.final_force_n >= 0.5);
    }

    #[test]
    fn injured_finger_fails_the_grasp_then_succeeds_after_healing() {
        // The Phase 4 demo, in miniature: injure, fail, wait for healing, succeed.
        let mut f = Finger::new(FingerParams::default());
        let object = Contact {
            angle_rad: 0.2,
            stiffness_nm_per_rad: 0.5,
        };
        f.injure();
        let first = run_grasp(&mut f, object, 2.0, 0.5, 0.5, 0.5, DT).unwrap();
        assert!(!first.success, "a severed tendon must not grip");

        // Wait for the healing timer to finish, then try again.
        while f.is_healing() {
            f.step(DT);
        }
        let second = run_grasp(&mut f, object, 2.0, 0.5, 0.5, 10.0, DT).unwrap();
        assert!(second.success, "{second:?}");
    }

    #[test]
    fn non_positive_dt_is_ignored() {
        let mut f = Finger::new(FingerParams::default());
        f.set_grip_force(2.0).unwrap();
        f.step(0.0);
        f.step(f64::NAN);
        assert_eq!(f.time_s(), 0.0);
        assert_eq!(f.angle_rad(), 0.0);
    }
}
