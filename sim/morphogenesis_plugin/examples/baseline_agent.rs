// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Runs the scripted baseline agent and prints its report.
//!
//! Run with `cargo run -p tpt-hands-morphogenesis-sim --example baseline_agent`.

use tpt_hands_morphogenesis_sim::baseline::{Episode, run_scripted};
use tpt_hands_morphogenesis_sim::{Contact, Finger, FingerParams};

fn main() {
    let object = Contact {
        angle_rad: 0.2,
        stiffness_nm_per_rad: 0.5,
    };
    let mut finger = Finger::new(FingerParams::default());
    match run_scripted(&mut finger, object, Episode::default()) {
        Ok(report) => {
            println!("first grasp ok:     {}", report.first_grasp_ok);
            println!("healing wait:       {:.3} s", report.healing_wait_s);
            println!("second grasp ok:    {}", report.second_grasp_ok);
            println!(
                "stiffness after:    {:.4} Nm/rad",
                report.final_stiffness_nm_per_rad
            );
        }
        Err(e) => {
            eprintln!("baseline failed: {e:?}");
            std::process::exit(1);
        }
    }
}
