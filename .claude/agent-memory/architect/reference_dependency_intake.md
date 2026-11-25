---
name: dependency-intake-check
description: How to vet a proposed Rust crate (version age, licence, features, cargo-deny) before any Cargo.toml edit, and the serde-saphyr licence trap found that way
metadata:
  type: reference
---

To vet crates for a dependency decision without touching the workspace:
- Versions: `curl -sS -A "<generic client>" https://crates.io/api/v1/crates/<name>/versions?per_page=15`
  into the scratchpad. The JSON gives `num`, `created_at`, `license`, `rust_version` and the feature map,
  where `default` lists the default features. The owner wants versions that are current and at least
  two weeks old, pinned with `=`.
- Graph check: make a scratch crate named `alz-io`, because deny.toml's rustix `wrappers` rule names it.
  Copy the repository's `deny.toml` next to it, list the existing workspace dependencies plus the
  candidates with their intended features, then run
  `mise exec -- cargo generate-lockfile --manifest-path <scratch>/Cargo.toml` and
  `mise exec -- cargo deny --manifest-path <scratch>/Cargo.toml check licenses bans sources advisories`
  from the repository root, so mise's pinned toolchain applies. `cargo tree -e normal -p <crate>`
  shows each crate's closure.

Found that way on 2026-11-23 (TASK-55.3 Q7): serde-saphyr 2.2.0's `deserialize` feature brings in
encoding_rs, licensed `(Apache-2.0 OR MIT) AND BSD-3-Clause`, which deny.toml does not allow.

**How to apply:** run this before writing any dependency decision, and record the versions, licences,
closure and cargo-deny result in its Context. See [[backlog-decision-cli-title-only]].

Std API stability (2026-11-23, R3 plan): the pinned toolchain's std source is at
`~/.rustup/toolchains/1.100.0-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/`. Grep it for
`#[unstable` before planning on a std API: `CommandExt::setsid` is unstable there (process_setsid,
issue 105376), while rustix 1.1.6's `process::setsid()` is a safe fn. Hence the R3 re-exec trampoline
for TASK-70 (no unsafe, decision-27). This was read from source, not run.

regex-lite 0.1.9 (decision-38, 2026-11-23): it has no runtime dependencies, and its features are `std` and `string`. A smoke test on Rust 1.100.0 showed that look-around, backreferences, `\p{..}`, a size limit above 64 KiB and a nest limit above 32 all fail to compile. `(?i)` folds ASCII letters only, and `\w`, `\d` and `\b` are ASCII. A scratch crate named `alz-core` with `cargo test -- --nocapture` printed the facts. regex 1.13.2 came out on 2026-11-18, too new; 1.13.1 is the fallback.
