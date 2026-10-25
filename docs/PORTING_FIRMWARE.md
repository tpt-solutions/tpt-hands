<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# Porting the edge firmware to another MCU

This is the guide for adding an MCU to `crates/tpt-hands-edge`. Read
[ARCHITECTURE.md](ARCHITECTURE.md) first, for the layers.

The edge crate is `no_std`. Its logic is plain arithmetic with no heap and no
clock, so it runs the same on the host and on a chip. What differs per MCU is the
HAL: the ADC, the force sensor, the transport and the timer. Those are not written
yet, and the guide says where they go.

## What already builds

These targets build the edge crate today (checked with `cargo build -p
tpt-hands-edge --target <target>`):

| Target | Chip | Notes |
| --- | --- | --- |
| `riscv32imc-unknown-none-elf` | ESP32-C6 | Compressed-only RISC-V. |
| `riscv32imac-unknown-none-elf` | ESP32-C6 | The C6 core. Use this one for the real chip. |
| `thumbv7em-none-eabihf` | STM32G4 | The reference family. |

Building is not the same as running. No code has run on hardware, and no
bring-up has been done.

## Steps

1. **Install the target.** `rustup target add <target>`.
2. **Build the edge crate for it.** It should build without changes. If it does
   not, the problem is in the crate, so fix it there and not in the port.
3. **Write the HAL for the chip.** The crate needs four things from the board.
   Each one is a small driver, behind a trait you define in the firmware crate.
   - **Pathway resistance.** An ADC reading per pathway, converted to ohms. Feed
     the result to `reflex::PathwayMonitor::sample`. Filtering goes before the
     monitor, not inside it.
   - **Grip force.** A force reading in newtons, fed to `reflex::GripReflex::on_force`.
   - **Telemetry and commands.** A transport for `tpt-link` frames. The frame
     codec is in `tpt-hands-proto` and works without `alloc` for decoding.
     Check the feature gates of `tpt-link` before you assume a transport module is
     available without `std`.
   - **Time.** A monotonic microsecond counter for the 1 kHz loop and for the
     healing timer, if the firmware runs it.
4. **Write the control loop.** One 1 kHz task: read the sensors, call the reflex
   functions, send any telemetry, and handle commands. Measure the jitter. The
   host benchmark (`cargo bench -p tpt-hands-edge --bench reflex_tick`) shows the
   logic cost, not the MCU's.
5. **Add the safe state.** The watchdog, the panic handler and the default
   outputs are not written. On a fault the relay must open, as the handshake
   requires (`docs/HANDSHAKE.md`). Do not ship without this.
6. **Test the port.** The reflex logic is tested on the host, so the port only
   needs tests for the HAL: the ADC conversion, the frame round trip over the
   real transport, and the timing.

## What is not ready

- The HAL trait layer (todo.md, Phase 1). Each port currently writes its own.
- The watchdog, panic handler and safe-state defaults.
- The 500 ms micro-calibration cycle on the device.
- The on-device pathway state machine's Hypertrophied handling beyond the host
  reference (`PathwayMonitor::reinforce` exists, but no firmware calls it yet).
- Flash and debug tooling (`probe-rs`, `espflash`) and a hardware-in-the-loop
  plan.
- The handshake on the device (`crates/tpt-hands-handshake` is host-side).

## Checklist for a new MCU

- [ ] Target installed, and the edge crate builds for it
- [ ] Pathway ADC driver, with conversion to ohms and a test
- [ ] Force sensor driver, with conversion to newtons and a test
- [ ] Transport driver, with a frame round trip over the real link
- [ ] 1 kHz loop with measured jitter, recorded
- [ ] Watchdog and safe state, tested by forcing a fault
- [ ] Relay opens on link loss (measured, not assumed)
- [ ] Flash and debug steps written down
