---
id: decision-34
title: >-
  SHA-256 comes from sha2 0.11.1 with default features off, in alz-core, for
  content hashes and cache names (owner, TASK-55.3)
date: '2026-11-23T05:24:00Z'
status: accepted
---
## Context

alz computes SHA-256 for the layer content hash (spec §5.3, its byte layout from TASK-65), for the
names of cache entries (`git-<sha256>`, `pathref@<sha256>`, §6) and for the hashes of generated files
in `layer.lock`. Hashing is pure, so it belongs in `alz-core` (decision-23). The owner approved sha2 on
2026-11-23 (TASK-55.3 notes, answer 2).

Options considered: sha2 (RustCrypto, pure Rust); ring and aws-lc-rs (C and assembly, heavier builds
for the four targets of decision-25); an alz-owned implementation, which is not an option for a hash.

Facts: sha2 0.11.1 was released on 2026-10-12. It is MIT OR Apache-2.0, has `rust-version` 1.88 (the pin
is 1.100.0) and `default = ["alloc", "oid"]`, plus `zeroize`. It adds `digest` 0.11, `block-buffer`,
`crypto-common`, `hybrid-array`, `typenum`, `cfg-if` and `cpufeatures`, which picks the SHA extensions
of the CPU at run time and uses `libc` on some targets. Its unsafe code stays inside the dependency, and
`unsafe_code = "forbid"` still holds in every alz crate. Checked on 2026-11-23 through the crates.io API and a scratch crate that depends on the five crates of TASK-55.3 next to clap 4.6.9, assert_cmd 2.2.2, rustix 1.1.6 or later and serde-saphyr =2.2.0, with this repository's `deny.toml`: `cargo deny check bans sources advisories` passed, and `licenses` passed for these five. The one licence failure in that graph is encoding_rs (`(Apache-2.0 OR MIT) AND BSD-3-Clause`), which serde-saphyr's `deserialize` feature brings in (decision-28), not this crate.

## Decision

`sha2 = { version = "=0.11.1", default-features = false }` in `[workspace.dependencies]`, used by
`alz-core` only.
- alz uses `Sha256` through the `Digest` trait.
- `alloc`, `oid` and `zeroize` stay off: alz hashes no secrets and needs no ASN.1 object ids.
- Lowercase hex is alz's own code; there is no hex crate.

## Consequences

- R2.7 (TASK-55.3.7) adds it with `ContentHash` and `FileDigest`. Tests use a standard SHA-256 vector
  and the test-side formula of R1.2 (`test/spec/content-hash.ts`).
- The backend may differ by CPU, but the output never does.
- Moving to another hash crate is a new decision.
- Raising the pin within the same major version is an update task: the new version must be at least two weeks old, and `cargo deny` and `cargo audit` must pass. A new major version, or another feature, is a new decision.
