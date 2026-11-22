---
id: decision-29
title: >-
  The alz binary uses clap 4.6.9 with derive for arguments and assert_cmd 2.2.2
  as a dev dependency for CLI tests (owner, TASK-55.1.3)
date: '2026-11-22T16:45:00Z'
status: accepted
---
## Context

doc-5 says every new dependency is a recorded decision. TASK-55.1.3 built the Cargo skeleton, and
the skeleton has the first two crates from outside the workspace:

- **clap** parses the arguments. doc-5's Rust standard names "`clap` derive", and decision-26
  assumes clap's help, usage errors and `--version`, parsed with `try_parse` so that alz renders
  and sanitises clap's errors itself.
- **assert_cmd** runs the built binary in tests. TASK-55.1.3's AC #1 asks for an `assert_cmd`
  test of `alz --version` and `alz --help`, and the gr-rust skill names it for CLI tests.

The standard library offers neither: `std::env::args_os` gives raw arguments with no parsing,
help or usage errors, and writing an argument parser would mean owning wording that decision-26
leaves to clap.

Checked on 2026-11-22 against crates.io:
- clap 4.6.9: released 2026-11-20, `MIT OR Apache-2.0`, MSRV 1.85, edition 2024, actively
  maintained.
- assert_cmd 2.2.2: released 2026-05-11, `MIT OR Apache-2.0`, MSRV 1.85.
- The RustSec advisory-db has no entry for clap.

Transitive weight in the skeleton's `Cargo.lock`:
- clap brings about 18 crates: `clap_builder`, `clap_derive`, `clap_lex`, the `anstream` and
  `anstyle` family, `colorchoice`, `strsim`, and at build time `heck`, `syn`, `quote` and
  `proc-macro2`.
- assert_cmd adds about 11 crates, for tests only.

## Decision

The workspace declares both in `[workspace.dependencies]`, and members use them with
`.workspace = true`:
- `clap = { version = "4.6.9", features = ["derive"] }`, a runtime dependency of the `alz` binary
  crate only. The binary parses with `try_parse` and renders clap's errors itself, sanitised
  (decision-26).
- `assert_cmd = "2.2.2"`, a dev dependency of the `alz` crate only, for tests that run the built
  binary.

`alz-core` and `alz-io` depend on neither.

## Consequences

- `cargo deny` (TASK-55.1.4) enforces the licence policy, advisories and duplicate versions for
  these crates and everything they pull in. A licence or advisory it refuses is fixed by a new
  decision, not by an exception added without one.
- Each further crate needs its own recorded decision, test-only and build-time crates included:
  `thiserror`, `proptest`, `insta`, `predicates`, `tempfile`, a hashing crate, and the crates
  already decided (rustix in decision-27, serde-saphyr in decision-28) when they are first added.
- clap's output wording follows its defaults, and clap upgrades can change help and usage text.
  The spec states only the stream and exit code for that text (decision-26), so such an upgrade
  is a reviewed dependency bump, not a spec change.
- Version bumps go through `Cargo.lock` in a reviewed change. A clap MSRV above the pinned
  toolchain would block the bump until the pin rises (decision-25).
