---
id: decision-38
title: >-
  Leak guard patterns and the secret scan use regex-lite 0.1.9 in alz-core, std
  and string only, with size and nest limits (owner, TASK-55.5)
date: '2026-11-23T12:37:00Z'
status: accepted
---
## Context

The owner's answer 2 in TASK-55.5 (2026-11-23) puts the leak guard (spec §9.2, `forbid_patterns`) and the built-in secret scan into R4, with a decision on a regex crate. The spec states the pattern syntax, and look-around and backreferences are refused with a clear error. The coordinator relayed the instruction to record the decision as accepted and to pick the smallest crate that works without look-around. The architect chose the crate on that basis.

`forbid_patterns` come from a layer's own `layer.yaml`, which is untrusted when the layer is remote or is the repository being built. So compiling and matching must be bounded. The TypeScript reference compiles them with JavaScript's `RegExp`, without the `u` flag and with `i` for a leading `(?i)`. In that mode `\d`, `\w` and `\b` are ASCII. The reference has no size limit.

Dependency intake (2026-11-23, crates.io API and a scratch crate checked from the repository root, so mise's toolchain applied):
- regex-lite 0.1.9, released 2026-02-03, is the current version and well over two weeks old. Licence `MIT OR Apache-2.0`, both in deny.toml's allow list. `rust-version` 1.65, below the 1.100.0 pin. Features: `std` and `string`, which are also the defaults, and no others. No runtime dependencies: `cargo tree -e normal` shows regex-lite alone.
- `cargo deny check licenses bans sources advisories` on the scratch graph: advisories, bans, licences and sources ok. The only warnings were for rules of crates not in the scratch graph.
- regex 1.13.2 was 5 days old, and 1.13.1 (2026-07-15) is acceptable by age. With default features off it still brings regex-automata and regex-syntax. It adds Unicode classes and a lazy DFA, which alz does not need here.
- regex-syntax alone is a parser without a matcher, so it is not enough.
- A smoke test of regex-lite 0.1.9 on Rust 1.100.0, with `size_limit(64 KiB)` and `nest_limit(32)`, gave these results:
  - These fail to compile: `(?=`, `(?!`, `(?<=`, `(?<!`, the backreferences `\1` and `\k<n>`, `\p{L}`, `a{1000}{1000}` (size limit) and 40 nested groups (nest limit).
  - `(?i)\bacme\b` matches `ACME corp` and not `acmeco`; `(?i)é` does not match `É` (ASCII case folding only).
  - Every token shape of the reference's secret scan compiles.

## Decision

- alz-core depends on `regex-lite = { version = "=0.1.9", default-features = false, features = ["std", "string"] }`. Only alz-core depends on it, so the leak guard and the secret scan stay pure.
- The pattern syntax of `forbid_patterns` is regex-lite's, and spec §9.2 states it:
  - inline flags such as a leading `(?i)`;
  - `\d`, `\s`, `\w` and `\b` are ASCII only;
  - case-insensitive matching is ASCII only;
  - no `\p{...}`, no look-around and no backreferences.
- An entry is refused with `leak-forbid`, naming its index and never its text, when it:
  - uses look-around or a backreference (alz finds these before compiling, so the message says so);
  - does not compile;
  - holds more than 512 characters;
  - compiles past the size limit (64 KiB) or the nest limit (32).
  A layer has at most 64 entries.
- The built-in secret scan compiles its fixed token shapes with the same crate, once, and a test proves that each one compiles. A finding names the file and the kind of token, never the text.

## Consequences

- R4.11 (TASK-55.5.11), the first user (the secret scan), adds the dependency to the workspace `Cargo.toml` and to alz-core. It also adds a deny.toml `[bans]` entry, `{ name = "regex-lite", wrappers = ["alz-core"] }`, and pins the version there as for the other pinned crates. R4.14 (TASK-55.5.14) uses the crate for forbid_patterns. The architect edits neither file.
- Matching is bounded: at worst O(m * n) per pattern and file, with `m` capped by the size limit and `n` by the 16 MiB file cap. The scan uses `is_match`, never `find_iter`, which is O(m * n^2).
- These changes are user-facing compared with the reference and are reference drift (decision-30): a pattern that used look-around, backreferences, `\p{...}` or non-ASCII case folding, or that breaks a limit, now fails the lint. This goes into the CHANGELOG with the cutover's minor bump.
- If Unicode classes or case folding are needed later, `regex` with default features off is the fallback. It needs a new decision that supersedes this one.


