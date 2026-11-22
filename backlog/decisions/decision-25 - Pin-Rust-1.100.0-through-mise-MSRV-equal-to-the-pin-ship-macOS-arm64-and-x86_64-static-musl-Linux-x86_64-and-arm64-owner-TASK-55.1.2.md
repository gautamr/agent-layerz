---
id: decision-25
title: >-
  Pin Rust 1.100.0 through mise, MSRV equal to the pin; ship macOS arm64 and
  x86_64, static musl Linux x86_64 and arm64 (owner, TASK-55.1.2)
date: '2026-11-22T16:27:00Z'
status: accepted
---
## Context

This is doc-5 choice 4. Checked on 2026-11-22:
- On the maintainer's machine the active stable toolchain is 1.99.1. `rustup check` reports
  1.100.0 (2026-11-09) as the current stable, and 1.100.0 is already installed.
- Rust currently comes from rustup, and `mise.toml` has no Rust entry. The personal rule says
  runtimes go through mise.
- Edition 2024 needs Rust 1.85 or later.

alz ships prebuilt binaries (R7) and has no library consumers. A lower MSRV would mainly help
people who build it from source with an older toolchain, at the cost of a second CI toolchain.
Options considered:
- an MSRV equal to the pin;
- the pin plus an MSRV a few releases back, with its own CI job;
- MSRV 1.85, the floor for edition 2024.

## Decision

Development and CI use Rust 1.100.0, pinned through mise (`mise.toml`). If `rust-toolchain.toml`
also exists (doc-6 §2 lists it for components and for rustup users), it names the same version,
never a floating `stable`. `rust-version` in `[workspace.package]` equals the pinned version, and
both rise together in one reviewed change. While they are equal, CI runs on the pin and needs no
separate MSRV job.

Release binaries are built for:
- `aarch64-apple-darwin`;
- `x86_64-apple-darwin`;
- `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`, statically linked so they run on
  any distribution.

Windows stays undecided, as before.

## Consequences

- TASK-55.1.3 adds the mise pin and `rust-version`. How mise and rustup choose a toolchain when
  both a mise pin and `rust-toolchain.toml` exist is not verified yet; that task checks it and
  keeps one source of the version.
- TASK-55.1.5 runs CI on the pin.
- R7 (TASK-55.8) builds the four targets. It has to choose how to build musl (for example
  cargo-zigbuild or cross) and to confirm each target's current Rust support tier; neither is
  verified here.
- People who build from source need the pinned stable or newer. Resolver 3's MSRV-aware fallback
  has nothing to do while the MSRV is the pin.
- Raising the pin is routine upkeep, done as its own change.
