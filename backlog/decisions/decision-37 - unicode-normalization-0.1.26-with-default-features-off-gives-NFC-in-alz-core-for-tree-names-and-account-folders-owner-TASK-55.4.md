---
id: decision-37
title: >-
  unicode-normalization =0.1.26 with default features off gives NFC in alz-core
  for tree names and account folders (owner, TASK-55.4)
date: '2026-11-23T07:57:00Z'
status: accepted
---
## Context

The spec compares several kinds of names in Unicode NFC:
- **Tree entries.** A `tree2@` entry is compared with its git tree in NFC, because HFS+ returns NFD and APFS keeps the form a name was written in.
- **Folding collisions.** A tree with two paths that are equal after NFC and case folding is refused (§6).
- **Account folders.** These names are matched after NFC and lowercasing, so a non-ASCII home on APFS cannot hide `~/.claude-x` behind another form (TASK-52, §9.1).

The standard library has no normalisation, and the TypeScript reference uses `String.prototype.normalize`. Normalising is pure, so it belongs in `alz-core` (decision-23).

The owner approved unicode-normalization =0.1.26 on 2026-11-23 (TASK-55.4 notes, answer 3).

Options considered:
- unicode-normalization (the unicode-rs project, which also maintains unicode-ident);
- icu_normalizer from ICU4X, which is a much larger tree: data providers, zerovec, yoke and more;
- an alz-owned table, which would need the composition data of the Unicode database and its tests.

Facts, checked on 2026-11-23 through the crates.io API and the scratch crate of decision-36:
- **Release and licence**: version 0.1.26 was released on 2026-10-16, more than two weeks ago. It is licensed `(MIT OR Apache-2.0) AND Unicode-3.0`. `deny.toml` already allows Unicode-3.0, for unicode-ident.
- **MSRV and features**: its `rust-version` is 1.36, below the 1.100.0 pin. Its only feature is `std`, which is in `default`.
- **Dependencies**: with `default-features = false` it adds one crate, `tinyvec` 1.13.3, licensed `Zlib OR Apache-2.0 OR MIT`.
- **Unicode version**: its tables are Unicode 18.0.0 (`UNICODE_VERSION` in `src/tables.rs`). Node's ICU may be older, so a name that uses a character assigned after Node's version could normalise differently in the two binaries. That is accepted reference drift, like the Cf table of TASK-55.3.
- **Supply-chain check**: `cargo deny check licenses bans sources advisories` passed with this repository's `deny.toml`.
- **Test**: under the 1.100.0 toolchain, a probe test passed. It showed that `"e\u{301}X"` NFC-normalised, lowercased and NFC-normalised again gives `"\u{e9}x"`, which is the TypeScript reference's `normalize("NFC").toLowerCase().normalize("NFC")`.

## Decision

`unicode-normalization = { version = "=0.1.26", default-features = false }` goes in `[workspace.dependencies]`, used by `alz-core` only.
- alz uses NFC only, through `UnicodeNormalization::nfc`.
- Case folding is std's `str::to_lowercase`, framed by NFC as in the reference.
- `std` stays off, because alz needs only the iterators.

## Consequences

- R3.5 (TASK-55.4.5) adds the dependency for account-folder names. R3.9 (TASK-55.4.9) uses it for the tree comparison and the collision rule.
- Tests cover NFD and NFC forms of the same name, and a pair that differs only in case.
- A later need for NFD, NFKC or NFKD is not covered and needs a new decision.
- Raising the pin within the same version line is an update task: the new version must be at least two weeks old, and `cargo deny` and `cargo audit` must pass. A new minor version, or another feature, is a new decision.
