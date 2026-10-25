// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Bring-up for the ESP32-P4. It prints a heartbeat over the USB-JTAG serial port,
//! and runs the host-tested reflex logic from tpt-hands-edge on the chip. A
//! passing run on silicon is the first evidence that the shared code works there.

#![no_std]
#![no_main]

use esp_hal::delay::Delay;
use esp_hal::main;
use esp_backtrace as _;
use esp_println::println;
use tpt_hands_edge::reflex::{PathwayEvent, PathwayMonitor, pick_route};

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    let _peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    println!("tpt-hands bring-up: ESP32-P4 alive");

    // Replay a tear on pathway 0 with the same thresholds the host tests use.
    let mut monitors = [PathwayMonitor::new(100.0, 50.0, 3); 2];
    let samples = [10.0, 10.0, 500.0, 500.0, 500.0, 10.0, 10.0, 10.0];
    let mut tick: u32 = 0;
    loop {
        for &ohms in &samples {
            let event = monitors[0].sample(ohms);
            if event != PathwayEvent::None {
                println!("tick {}: pathway 0 event {:?}", tick, event);
            }
            tick = tick.wrapping_add(1);
            println!("tick {}: route {:?}", tick, pick_route(&monitors, &[0, 1]));
            delay.delay_millis(100);
        }
        monitors[0] = PathwayMonitor::new(100.0, 50.0, 3);
        println!("cycle complete, restarting");
        delay.delay_millis(1000);
    }
}
