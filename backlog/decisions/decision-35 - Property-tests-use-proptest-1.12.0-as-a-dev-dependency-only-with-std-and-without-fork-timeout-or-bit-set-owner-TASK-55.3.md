---
id: decision-35
title: >-
  Property tests use proptest 1.12.0 as a dev-dependency only, with std and
  without fork, timeout or bit-set (owner, TASK-55.3)
date: '2026-11-23T05:25:00Z'
status: accepted
---
## Context

decision-22 and doc-5 name `proptest`. decision-26 (amended) requires property tests for the text
classes, and decision-28 requires one for YAML diagnostics. doc-6 §3.5 states the merge laws as property
tests. The owner approved proptest as a dev-dependency on 2026-11-23 (TASK-55.3 notes, answer 2).

Facts: proptest 1.12.0 was released on 2026-10-10. It is MIT OR Apache-2.0 and has `rust-version` 1.88.
Its default features are `std`, `fork`, `timeout` and `bit-set`.
- `fork` brings in `rusty-fork` and `tempfile`, and `tempfile` brings in `rustix`, which `deny.toml`
  restricts to `alz-io` and `tempfile`. `timeout` brings in `wait-timeout`.
- With only `std` it adds `bitflags`, `num-traits`, `rand` 0.10, `rand_core`, `rand_xorshift`,
  `chacha20`, `getrandom` 0.4, `regex-syntax`, `unarray`, `core_detect` and `cfg-if`.

Checked on 2026-11-23 through the crates.io API and a scratch crate that depends on the five crates of TASK-55.3 next to clap 4.6.9, assert_cmd 2.2.2, rustix 1.1.6 or later and serde-saphyr =2.2.0, with this repository's `deny.toml`: `cargo deny check bans sources advisories` passed, and `licenses` passed for these five. The one licence failure in that graph is encoding_rs (`(Apache-2.0 OR MIT) AND BSD-3-Clause`), which serde-saphyr's `deserialize` feature brings in (decision-28), not this crate.

## Decision

`proptest = { version = "=1.12.0", default-features = false, features = ["std"] }` in
`[workspace.dependencies]`, referenced only from the `[dev-dependencies]` of the crates that test with it,
never as a normal dependency.
- `fork`, `timeout`, `bit-set` and `attr-macro` stay off.
- Tests use the default number of cases unless the test says why it changes them.
- Regression files (`proptest-regressions/`) are committed.
- Strategies live in the test modules, or in `alz-test-support` once several crates share one.

## Consequences

- R2.2 (TASK-55.3.2) adds it first, for the text-class laws.
- `cargo deny` checks dev-dependencies too, so licences and duplicate versions cover it.
- Review checks that `cargo tree -e normal` never lists it.
- Fuzzing (doc-6's later `fuzz/` crate) is a separate decision.
- Raising the pin within the same major version is an update task: the new version must be at least two weeks old, and `cargo deny` and `cargo audit` must pass. A new major version, or another feature, is a new decision.
