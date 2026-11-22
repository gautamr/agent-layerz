---
id: decision-23
title: >-
  Rust workspace per doc-6: alz-core, alz-io, alz, alz-test-support; only alz-io
  reaches fs, process or env (owner, TASK-55.1.2)
date: '2026-11-22T16:27:00Z'
status: accepted
---
## Context

doc-5 lists the repository layout and the crate layout as R0 choices. doc-6 §2 proposes a Cargo
workspace in this repository and four crates. The gr-rust skill says to start with one crate and
split only for a boundary the compiler enforces, a second consumer, or build time. In doc-6 only
`alz-core` bans effectful calls. The `alz` binary could still call `std::fs` or
`std::process::Command` directly, so "every effect lives in `alz-io`" would only be a convention.
Checked with clippy 1.99 in a scratch workspace (TASK-55.1.2 notes): a crate's own `clippy.toml`
replaces the workspace one and is not merged with it. Options considered: doc-6 as written; doc-6
with the binary also banned from effects; two crates (core and binary).

## Decision

A Cargo workspace sits at the repository root, next to the frozen TypeScript code until cutover
(R8). It is a virtual manifest with `members = ["crates/*"]`, `resolver = "3"`, and
`[workspace.package]`, `[workspace.lints]` and `[workspace.dependencies]` as in doc-6 §2.

The crates, each in a flat folder under `crates/` named after it:
- `alz-core`: a library with no I/O. It does not depend on `alz-io`, and its `clippy.toml`
  disallows the `std::fs`, `std::process` and `std::env` functions.
- `alz-io`: a library that performs every effect, behind capability types (doc-6 §3.4).
- `alz`: the binary, with clap, wiring, diagnostic rendering and exit codes. Its `clippy.toml`
  disallows the same `std::fs`, `std::process` and `std::env` functions (arguments are read
  through clap), so only `alz-io` reaches the file system, processes or the environment.
- `alz-test-support`: `publish = false`.

Every crate-level `clippy.toml` repeats the workspace's test allowances, because it replaces the
root file. `xtask/` (the parity run, release preparation) and `fuzz/` (cargo-fuzz targets) are
added by the first task that needs them, not by the skeleton; `xtask` then joins `members`.
`test/fixtures/` stays shared by both implementations until R8. Inside each crate the gr-rust
module rules apply: a `foo.rs` next to `foo/` instead of `mod.rs`, items re-exported one by one,
no prelude.

## Consequences

- TASK-55.1.3 creates the four crates, the root `clippy.toml` and the two crate files with the
  bans. Its AC #3 check (a deliberate `std::fs` call fails clippy) applies to the binary as well.
- The `alz` binary's tests in `crates/alz/tests/` fall under the same ban. They set up files
  through `alz-test-support`, or through an `#[expect(clippy::disallowed_methods, reason = "…")]`
  that states why.
- A fifth crate needs an enforced boundary, a second consumer or a measured build-time reason,
  and is recorded as a new decision.
- doc-6 §2 is updated to match: the binary's `clippy.toml`, and xtask and fuzz added when needed.
