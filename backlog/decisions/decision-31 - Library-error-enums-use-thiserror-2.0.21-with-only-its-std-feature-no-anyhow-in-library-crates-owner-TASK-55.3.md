---
id: decision-31
title: >-
  Library error enums use thiserror 2.0.21 with only its std feature; no anyhow
  in library crates (owner, TASK-55.3)
date: '2026-11-23T05:24:00Z'
status: accepted
---
## Context

doc-5's Rust standard and decision-22 name `thiserror` for typed errors in library code, one error type
per boundary and no `anyhow` in library crates. Every new dependency is a recorded decision
(`.claude/rules/rust.md`). R2 (TASK-55.3) brings the first error enums: the YAML, schema, lock and source
errors in `alz-core` and the read error in `alz-io`. The owner approved the crate on 2026-11-23 (TASK-55.3
notes, answer 2).

Options considered: `thiserror`; hand-written `Display` and `Error` impls; `anyhow` or `eyre`, which doc-5
rules out for libraries.

Facts: thiserror 2.0.21 was released on 2026-09-23. It is MIT OR Apache-2.0, has `rust-version` 1.77
and the features `default = ["std"]` and `std`. It adds the proc macro `thiserror-impl`, with
`proc-macro2`, `quote`, `syn` and `unicode-ident` (Unicode-3.0, already allowed), all at build time only.
Checked on 2026-11-23 through the crates.io API and a scratch crate that depends on the five crates of TASK-55.3 next to clap 4.6.9, assert_cmd 2.2.2, rustix 1.1.6 or later and serde-saphyr =2.2.0, with this repository's `deny.toml`: `cargo deny check bans sources advisories` passed, and `licenses` passed for these five. The one licence failure in that graph is encoding_rs (`(Apache-2.0 OR MIT) AND BSD-3-Clause`), which serde-saphyr's `deserialize` feature brings in (decision-28), not this crate.

## Decision

`thiserror = { version = "=2.0.21", default-features = false, features = ["std"] }` in
`[workspace.dependencies]`, used by `alz-core` and `alz-io`, and by the binary only if it needs an error
type of its own.
- Error enums derive `Error`, and `#[from]` converts between boundaries.
- The `#[error("…")]` text is for developers (tests, debug output) and never holds a field that carries
  input text. What a person sees comes only from `Diagnostic` and `Message` (R2.3, decision-26), never
  from an error's `Display`.
- No `anyhow` or `eyre` in any crate.

## Consequences

- The first subtask that defines an error enum adds it (R2.4, TASK-55.3.4, as planned).
- Review checks that no `#[error]` text interpolates a field holding repository content.
- Raising the pin within the same major version is an update task: the new version must be at least two weeks old, and `cargo deny` and `cargo audit` must pass. A new major version, or another feature, is a new decision.
