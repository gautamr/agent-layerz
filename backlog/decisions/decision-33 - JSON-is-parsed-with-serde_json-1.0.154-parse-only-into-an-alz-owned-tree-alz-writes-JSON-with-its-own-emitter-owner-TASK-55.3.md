---
id: decision-33
title: >-
  JSON is parsed with serde_json 1.0.154, parse only, into an alz-owned tree;
  alz writes JSON with its own emitter (owner, TASK-55.3)
date: '2026-11-23T05:24:00Z'
status: accepted
---
## Context

`layer.lock` is JSON and untrusted (spec §5.3), and R4 and R5 read the JSON files of targets and
passthrough. JSON that alz writes must be byte-identical to the reference's `stableJson`, with keys in
code point order (§6, TASK-64). The owner approved serde_json, for parsing only, on 2026-11-23
(TASK-55.3 notes, answer 2).

Options considered: serde_json to parse, with alz's own emitter; serde_json for both; an alz-owned parser
and emitter, which would be more security-critical code to own.

Facts: serde_json 1.0.154 was released on 2026-10-11. It is MIT OR Apache-2.0, has `rust-version` 1.71
and `default = ["std"]`. Its optional features are `alloc`, `arbitrary_precision`, `float_roundtrip`,
`preserve_order`, `raw_value` and `unbounded_depth`. It adds `itoa`, `memchr`, `serde_core` and `zmij`.
Without `unbounded_depth` the parser stops at nesting depth 128. Its `Value` keeps the last of two
duplicate keys, and without `preserve_order` it sorts keys. Checked on 2026-11-23 through the crates.io API and a scratch crate that depends on the five crates of TASK-55.3 next to clap 4.6.9, assert_cmd 2.2.2, rustix 1.1.6 or later and serde-saphyr =2.2.0, with this repository's `deny.toml`: `cargo deny check bans sources advisories` passed, and `licenses` passed for these five. The one licence failure in that graph is encoding_rs (`(Apache-2.0 OR MIT) AND BSD-3-Clause`), which serde-saphyr's `deserialize` feature brings in (decision-28), not this crate.

## Decision

`serde_json = { version = "=1.0.154", default-features = false, features = ["std"] }` in
`[workspace.dependencies]`, used by `alz-core` only.
- Parse only. alz reads with `serde_json::Deserializer` and its own visitor (decision-32), building an
  alz JSON tree that keeps document order and refuses a duplicate key. `serde_json::Value` is not used.
- Input is capped at 1 MiB and read as strict UTF-8 before parsing. The depth limit of 128 stays.
- A diagnostic comes from the error's category, line and column only, never from serde_json's
  `Display` text, as for YAML (decision-28).
- `arbitrary_precision`, `float_roundtrip`, `preserve_order`, `raw_value` and `unbounded_depth` stay off (amended below: `float_roundtrip` is on).
- alz writes JSON with its own stable emitter (`alz-core/src/render/json.rs`, R2.7), tested byte for
  byte against the reference. serde_json's serialiser is not used.

**Amended 2026-11-23 (owner, R5, TASK-55.6).** The `float_roundtrip` feature is enabled:
`serde_json = { version = "=1.0.154", default-features = false, features = ["std", "float_roundtrip"] }`.
Without it serde_json can read a decimal float up to one unit in the last place away from the nearest
double, while JavaScript's `JSON.parse` always gives the nearest double. `alz build --adopt` turns the
numbers of a hand-written `.claude/settings.json` and `.mcp.json` into layer YAML, and those bytes must
match the reference's. With the feature every JSON float alz reads is the nearest double (spec §4.3
"Adoption, in detail"). The feature adds no crate: in 1.0.154 it is `float_roundtrip = []` and selects
serde_json's own exact float parser (read from the crate's `Cargo.toml` and `src/de.rs`). The other four
features stay off.

## Consequences

- R2.7 (TASK-55.3.7) adds it with the lock model.
- The spec states the lock's reading rules: 1 MiB, nesting depth 128, a duplicate key makes the lock
  invalid (TASK-55.3.1).
- The pull request that adds it also adds a `[bans]` feature rule for the features above to `deny.toml`.
- **Amended 2026-11-23 (owner, R5, TASK-55.6).** The pull request that enables `float_roundtrip` removes
  it from the `deny` list of serde_json's feature rule in `deny.toml`, keeps the other four there, and adds
  tests: JSON floats whose nearest double the default parser misses are read exactly, and adopting them
  writes the reference's YAML bytes.
- Raising the pin within the same major version is an update task: the new version must be at least two weeks old, and `cargo deny` and `cargo audit` must pass. A new major version, or another feature, is a new decision.
