---
id: decision-32
title: >-
  alz uses serde 1.0.229 without derive, only to write its own visitors for
  serde-saphyr and serde_json (owner, TASK-55.3)
date: '2026-11-23T05:24:00Z'
status: accepted
---
## Context

decision-28 reads YAML into alz's own tree through serde-saphyr's `deserialize_any` and never derives
`Deserialize` on domain types, so a quoted `"true"` stays text. JSON is read the same way (decision-33).
Writing those visitors needs the `serde` crate as a direct dependency. The owner approved serde without
derive on 2026-11-23 (TASK-55.3 notes, answer 2).

Facts: serde 1.0.229 was released on 2026-07-18. It is MIT OR Apache-2.0, has `rust-version` 1.56 and the
features `default = ["std"]`, `std`, `alloc`, `derive`, `rc` and `unstable`. Since 1.0.220 it is a facade
over `serde_core`. serde-saphyr 2.2.0 requires `serde = "1"`. Without `derive` it adds only `serde_core`.
Checked on 2026-11-23 through the crates.io API and a scratch crate that depends on the five crates of TASK-55.3 next to clap 4.6.9, assert_cmd 2.2.2, rustix 1.1.6 or later and serde-saphyr =2.2.0, with this repository's `deny.toml`: `cargo deny check bans sources advisories` passed, and `licenses` passed for these five. The one licence failure in that graph is encoding_rs (`(Apache-2.0 OR MIT) AND BSD-3-Clause`), which serde-saphyr's `deserialize` feature brings in (decision-28), not this crate.

## Decision

`serde = { version = "=1.0.229", default-features = false, features = ["std"] }` in
`[workspace.dependencies]`, used by `alz-core` only.
- The `derive` feature is off, and no type in the workspace derives or implements `Serialize` or
  `Deserialize`. alz implements `Visitor` by hand for its YAML tree and its JSON tree, and turns those
  trees into domain types in its own code.
- `rc` and `unstable` stay off. Nothing in alz serialises through serde: generated YAML and JSON come
  from alz's own emitters (decision-28, decision-33).

## Consequences

- R2.4 (TASK-55.3.4) adds it with the YAML visitor.
- The pull request that adds it also adds a `[bans]` feature rule for serde's `derive` to `deny.toml`, so
  feature unification from another crate cannot turn it on unnoticed. If a dependency ever needs it, that
  is a new decision.
- Raising the pin within the same major version is an update task: the new version must be at least two weeks old, and `cargo deny` and `cargo audit` must pass. A new major version, or another feature, is a new decision.
