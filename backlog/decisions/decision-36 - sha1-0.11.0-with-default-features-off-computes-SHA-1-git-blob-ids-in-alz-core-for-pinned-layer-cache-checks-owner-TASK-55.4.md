---
id: decision-36
title: >-
  sha1 =0.11.0 with default features off computes SHA-1 git blob ids in alz-core
  for pinned-layer cache checks (owner, TASK-55.4)
date: '2026-11-23T07:57:00Z'
status: accepted
---
## Context

A pinned `path:` layer is cached as `tree2@<tree id>`. The entry is written from the tree's own blobs, and checked again on every read: each file must have the blob id that git computes over its bytes (spec §6). decision-24 says blob ids are computed in Rust, not by git, and names a SHA-1 crate as a dependency decision of its own. Most repositories use SHA-1 object ids. SHA-256 repositories are covered by sha2 (decision-34). Hashing is pure, so it belongs in `alz-core` (decision-23).

The owner approved sha1 =0.11.0 on 2026-11-23 (TASK-55.4 notes, answer 3).

Options considered:
- sha1 (RustCrypto, pure Rust, the same family as sha2);
- `git hash-object`, which decision-24 rules out;
- ring or aws-lc-rs: C and assembly, and ring has no SHA-1 for general use;
- an alz-owned SHA-1, which is not an option for a hash.

Facts, checked on 2026-11-23 through the crates.io API and a scratch crate:
- **Release and licence**: sha1 0.11.0 was released on 2026-03-27, which is more than two weeks ago. It is `MIT OR Apache-2.0`.
- **MSRV and features**: its `rust-version` is 1.85, below the 1.100.0 pin. Its features are `default = ["alloc", "oid"]`, plus `zeroize`.
- **Dependencies**: with `default-features = false` it adds only crates that sha2 0.11.1 already brings: `digest` 0.11.3, `block-buffer`, `crypto-common`, `hybrid-array`, `typenum`, `cfg-if` and `cpufeatures` (with `libc`). `cargo tree -d` showed no duplicate versions.
- **Supply-chain check**: the scratch crate was named `alz-io` and depended on this repository's workspace dependencies, with rustix's `process` feature added, plus sha1 and unicode-normalization (decision-37). Against this repository's `deny.toml`, `cargo deny check licenses bans sources advisories` printed "advisories ok, bans ok, licenses ok, sources ok".
- **Test**: under the 1.100.0 toolchain, a probe test computed the git id of the empty blob, `e69de29bb2d1d6434b8b29ae775ad8c2e48c5391`, from `blob 0\0`, and it passed.

## Decision

`sha1 = { version = "=0.11.0", default-features = false }` goes in `[workspace.dependencies]`, used by `alz-core` only.
- alz uses `Sha1` through the `Digest` trait, in `git/blob_id.rs`. The algorithm follows the commit's `ObjectId` kind: SHA-1 for 20 bytes, sha2's SHA-256 for 32.
- `alloc`, `oid` and `zeroize` stay off. alz hashes no secrets and needs no ASN.1 object ids.
- SHA-1 is used only to compute git's own object ids. It is never used for any check that alz defines itself. Content hashes and cache names stay SHA-256 (decision-34).

## Consequences

- R3.9 (TASK-55.4.9) adds the dependency with `BlobId`, tested against git's vectors for the empty blob in SHA-1 and SHA-256 and against `git hash-object` output in its integration tests.
- SHA-1 collisions are known, so a SHA-1 blob id proves only what git itself trusts. alz relies on it to match what the repository holds, not as a guard against the repository's owner, which is spec §9.3's position.
- Moving to another hash crate is a new decision.
- Raising the pin within the same minor line is an update task: the new version must be at least two weeks old, and `cargo deny` and `cargo audit` must pass. A new major or minor version, or another feature, is a new decision.
