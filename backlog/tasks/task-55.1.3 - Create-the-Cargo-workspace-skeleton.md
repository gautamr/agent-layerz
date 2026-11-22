---
id: TASK-55.1.3
title: Create the Cargo workspace skeleton
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-22T15:52:00Z'
updated_date: '2026-11-22T17:14:00Z'
labels:
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.1.2
references:
  - decision-29
parent_task_id: TASK-55.1
priority: high
ordinal: 61390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From the recorded layout: root Cargo.toml (resolver 3, workspace.package with edition 2024, rust-version and licence, workspace.dependencies, and the workspace.lints table from gr-rust references/layout-and-tooling.md), rust-toolchain.toml, clippy.toml, and the crates alz-core (no I/O; its clippy.toml bans std::fs, std::process and std::env), alz-io, alz (clap derive binary) and alz-test-support (publish = false). No domain code yet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 alz --version and alz --help work from cargo run, and an assert_cmd test proves both
- [x] #2 mise run rust:check passes with clippy pedantic and the restriction lints at -D warnings and no suppressions
- [x] #3 A test or lint proves alz-core cannot call std::fs (for example a deliberate call fails clippy, recorded in the task notes and removed)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add rust = "1.100.0" to mise.toml; verify what mise and rustup do with and without rust-toolchain.toml.
2. Root Cargo.toml (resolver 3, workspace.package, workspace.dependencies, workspace.lints), root clippy.toml, .gitignore target/.
3. Crates alz-core, alz-io, alz, alz-test-support, each with [lints] workspace = true; clippy.toml with bans on alz-core and alz.
4. Failing tests first: assert_cmd tests for --version and --help in crates/alz/tests.
5. Prove the bans: deliberate std::fs call in alz-core fails clippy; record output; remove it.
6. Run rust:check, test, lint; confirm npm tooling ignores the new folders.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation notes (skeleton, no domain code).

mise and rustup (decision-25 left this unverified), checked on mise 2026.9.1 with rustup 1.29.3:
- `rust = { version = "1.100.0", profile = "minimal", components = "rustfmt,clippy" }` in mise.toml uses rustup underneath. `mise install rust` reused the existing rustup toolchain 1.100.0 (CARGO_HOME ~/.cargo, RUSTUP_HOME ~/.rustup).
- Inside mise (mise x, mise run, shell activation) mise exports RUSTUP_TOOLCHAIN=1.100.0. rustup gives that variable priority over rust-toolchain.toml: with a deliberate rust-toolchain.toml naming 1.91.0, `mise x -- rustc -V` still said 1.100.0 and `rustup show active-toolchain` said "overridden by environment variable RUSTUP_TOOLCHAIN".
- Outside mise, with ~/.cargo/bin first on PATH, the same file made rustc 1.91.0 (rustup obeys the file).
- So rust-toolchain.toml would be a second source of the version that mise silently ignores: they can only drift apart. Decision: no rust-toolchain.toml; mise.toml is the one source (decision-25 "keep one source of the version"). Cargo's rust-version = 1.100.0 makes an older toolchain fail with a clear error for anyone building without mise. If rustup-only users matter later, add a rust-toolchain.toml with the same version in the same change that raises the pin, or generate it.
- Cargo 1.100 warns on unused dependencies and unused workspace dependencies (cargo::unused_dependencies, cargo::unused_workspace_dependencies); with -D warnings that matters. Internal crates are therefore not in workspace.dependencies yet: add each with its first user.

Ban evidence (AC #3), clippy 1.100.0, `cargo clippy -p <crate> --all-targets -- -D warnings`, deliberate calls removed afterwards:
alz-core, with std::env::var, std::process::Command::new and std::fs::read_to_string added:
  error: use of a disallowed method `std::env::var`                 --> crates/alz-core/src/lib.rs:10:17
  error: use of a disallowed method `std::process::Command::new`    --> crates/alz-core/src/lib.rs:11:13
  error: use of a disallowed method `std::fs::read_to_string`       --> crates/alz-core/src/lib.rs:12:5
  error: could not compile `alz-core` (lib) due to 3 previous errors
alz (decision-23 consequence), with std::fs::read_to_string added to main.rs:
  error: use of a disallowed method `std::fs::read_to_string`       --> crates/alz/src/main.rs:23:5
Control: the same std::fs::read_to_string in alz-io raised no disallowed_methods error (only an unrelated pedantic missing_errors_doc), so the ban is specific to the two crates.
These are manual, one-off proofs; no automated test keeps them. A trybuild-style check could be a later task if wanted.

Behaviour (decision-26): `alz --version` and `-V` print "alz <version>" on stdout, exit 0; `--help`/`-h` print help on stdout, exit 0; no arguments print help on stderr, exit 2 (arg_required_else_help); usage errors go to stderr, exit 2. main uses Cli::try_parse; report.rs renders clap's error through terminal_text::printable, which removes control characters (except line feed) and the common Unicode format characters (bidi, zero-width, BOM, tag characters). Observed: off a terminal clap already strips CSI sequences itself; \r and U+202E reach the output unless alz removes them (covered by the integration test and unit tests). The format-character list is a hand-written Cf subset, not the full Unicode table; the spec's "Output and exit codes" task should settle it.

Dependencies added, all MIT OR Apache-2.0: clap 4.6.9 (derive) runtime; assert_cmd 2.2.2 dev. No dependency decision names either crate directly: decision-26 records clap's behaviour (4.6.9) and doc-5 says "CLI with clap derive"; assert_cmd appears in doc-5/doc-6 as the test tool. .claude/rules/rust.md says every new dependency is a recorded decision, so the owner should confirm those two cover it or record one. No thiserror, anyhow, predicates or serde yet.

Choices to review: only alz-test-support has publish = false (as the task says); the other three crates could be published by accident, consider publish = false everywhere. Crate clippy.toml files for alz-core and alz repeat the four test allowances plus the ban list (functions of std::fs, std::fs::File, OpenOptions, std::path::Path fs methods, std::process::Command::new/exit/abort/id, std::env args/vars/current_dir/etc). Cargo.lock is committed. `target/` added to .gitignore; biome, tsc and vitest were unaffected (lint and test pass).

Owner (session): publish = false on all four crates; clap and assert_cmd recorded in one dependency decision. Orchestrator reran mise run rust:check after the publish change: pass, 3 unit + 5 assert_cmd tests, no warnings.

Review fixes (implementer).
- Sanitiser: clap's error context (InvalidArg, InvalidValue, Suggested* and every other String/Strings/StyledStr value except Usage) is sanitised with strip_untrusted (Cc incl. LF, U+2028/2029, full Cf table) before clap renders it, so only clap's own line feeds survive; the rendered text then passes a last guard (strip_untrusted_keeping_line_feeds). The Cf table is crates/alz/src/terminal_text/format_characters.rs, generated from node's \p{Cf} (Unicode 17.0) by 'node crates/alz/tools/untrusted-code-points.mjs', which also writes crates/alz/tests/fixtures/untrusted-code-points.txt from isControl||isFormat of src/core/sanitize.ts (bundled with esbuild). A unit test compares the Rust predicate with that fixture over every code point. Regression tests: report.rs unit tests plus assert_cmd cases for LF (forged 'error [fake]' line), U+2028/2029 and U+0890/U+110BD/U+1D173. With sanitise_context disabled, the LF and escape tests fail. Behaviour note: an ESC inside an argument now leaves its '[31m' tail (as the TS reference does); before, clap's own ANSI stripping removed the whole sequence.
- escape_for_decision (TS quoteEscaped) not added: nothing in the binary calls it yet and an unused function fails clippy -D warnings. Add it with the task that prompts on untrusted text.
- Panic: main installs a hook that prints one fixed line (no payload, no location) and exit_code_of catches the unwind at the process boundary and returns 70. Unit-tested (exit code, fixed message); an end-to-end panic test is not possible without a test-only trigger in the binary.
- Output write failure: report::Rendered::emit flushes, says 'alz: cannot write its output: <io error>' on stderr when it can, and exits 1 (ordinary failure; 70 stays for bugs). Tested with a failing writer and end to end with a stdout pipe whose reader is closed (std::io::pipe; the one test that starts the binary itself, with #[expect] on the banned type).
- clippy.toml (alz-core and alz): added disallowed-types (File, OpenOptions, DirBuilder, process::Command, TcpStream, TcpListener, UdpSocket), methods DirBuilder::new, std::fs::soft_link, std::os::unix::fs::{symlink,chown,lchown,fchown?,chroot}, and in alz-core only SystemTime::now, Instant::now, io::{stdin,stdout,stderr}. File::create_buffered/open_buffered are still unstable (feature file_buffered) on 1.100, so not listed. Each file names its twin and the repeated root allowances. Proof: a throwaway module calling every banned item, in both crates, produced 'use of a disallowed method/type' for each (File, OpenOptions, DirBuilder, Command incl. the use line, Tcp*, Udp, DirBuilder::new, soft_link, symlink, chown, lchown, chroot; alz-core also SystemTime::now, Instant::now, stdin, stdout, stderr; the alz crate correctly does not flag the standard streams). Module removed afterwards. fchown is in the list but was not in the throwaway module.
- tests_outside_test_module: comment in command_line.rs on why tests sit in mod tests.

Correction to the note above: fchown is listed in both clippy.toml files (clippy reports no unresolved path for it); it was the only listed item the throwaway module did not call.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Cargo workspace skeleton per decision-23/25/26/29: virtual manifest (resolver 3, edition 2024, rust-version 1.100.0 = mise pin, gr-rust workspace lints), crates alz-core, alz-io, alz (clap derive, try_parse, sanitised clap output, exit 0/2/1) and alz-test-support, all publish = false. clippy.toml bans std::fs/process/env in alz-core and alz (manual proof in notes). No rust-toolchain.toml: mise exports RUSTUP_TOOLCHAIN, which overrides it, so mise.toml is the single source. Checks: mise run rust:check pass (3 unit + 5 assert_cmd tests), mise run lint pass, mise run test 840 passed. Follow-ups: the spec's Output and exit codes section settles the sanitiser's format-character list; internal crates join workspace.dependencies at their first user.
<!-- SECTION:FINAL_SUMMARY:END -->
