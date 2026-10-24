// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Cost of one reflex tick on the host: three pathway samples, a route pick and
//! one grip sample. The edge runs this at 1 kHz, so one tick must fit well
//! inside the 1 ms budget. Host numbers are only a rough guide to the MCU.
//!
//! Run with `cargo bench -p tpt-hands-edge`.

use std::hint::black_box;
use std::time::Instant;

use tpt_hands_edge::reflex::{GripReflex, PathwayMonitor, pick_route};

/// Period of the edge control loop, in nanoseconds (1 kHz).
const TICK_BUDGET_NS: f64 = 1_000_000.0;

/// Runs `f` for `iters` iterations and returns the mean time per iteration.
fn time_per_iter(iters: u64, f: &mut impl FnMut(u64)) -> f64 {
    let start = Instant::now();
    for i in 0..iters {
        f(i);
    }
    start.elapsed().as_secs_f64() * 1e9 / iters as f64
}

/// Best of several runs. The minimum is the least noisy estimate on a shared
/// machine.
fn best_of(runs: usize, iters: u64, mut f: impl FnMut(u64)) -> f64 {
    (0..runs)
        .map(|_| time_per_iter(iters, &mut f))
        .fold(f64::INFINITY, f64::min)
}

fn main() {
    let iters = 2_000_000;
    let runs = 5;

    // Healthy pathways, resistance well under the tear threshold, so the
    // monitors stay Intact and the route pick takes the common path.
    let mut monitors = [PathwayMonitor::new(100.0, 50.0, 3); 3];
    let mut grip = GripReflex::new(30.0, 5.0, 2.0);
    let preference = [0usize, 1, 2];

    let tick = |i: u64| {
        let r = 20.0 + (i % 7) as f32;
        for m in monitors.iter_mut() {
            black_box(m.sample(r));
        }
        black_box(pick_route(&monitors, &preference));
        black_box(grip.on_force(12.0 + (i % 5) as f32));
    };
    let ns_healthy = best_of(runs, iters, tick);

    // Worst case for the route search: every pathway severed, so the search
    // visits all entries and then reports no route.
    let mut severed = [PathwayMonitor::new(100.0, 50.0, 1); 3];
    for m in severed.iter_mut() {
        m.sample(500.0);
    }
    let ns_all_severed = best_of(runs, iters, |_| {
        black_box(pick_route(black_box(&severed), black_box(&preference)));
    });

    // The monitor's tear path, debounced with a single sample.
    let ns_tear = best_of(runs, iters, |i| {
        let mut m = PathwayMonitor::new(100.0, 50.0, 1);
        let r = if i % 2 == 0 { 500.0 } else { 10.0 };
        black_box(m.sample(r));
    });

    println!("reflex tick (3 monitors + route + grip)");
    report("healthy", ns_healthy);
    report("all pathways severed (route search)", ns_all_severed);
    report("monitor tear + reset", ns_tear);
    println!(
        "  budget use at 1 kHz: {:.4}% of 1 ms (host, not MCU)",
        ns_healthy / TICK_BUDGET_NS * 100.0
    );
}

fn report(label: &str, ns: f64) {
    println!("  {label:<40} {ns:>8.1} ns/op");
}
