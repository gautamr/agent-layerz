---
id: TASK-55.3.10
title: >-
  R2.10: alz lint of a layer and a stack manifest, through a no-follow capped
  read
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
updated_date: '2026-11-23T06:46:00Z'
labels:
  - area:rust
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.3.6
  - TASK-55.3.7
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 99390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Spec §10 `alz lint [--layer <path>]`, R2.1's "Output and exit codes", decision-27 for the read.

- alz-io: `crates/alz-io/src/fs.rs` with `fs/read_only_dir.rs`. `ReadOnlyDir::open(&Path)` for a directory the person named (`--layer` or the working directory) and `ReadOnlyDir::read_file(&SafeRelPath, cap)`: one component, `O_RDONLY | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW`; `fstat` on that handle must show `S_ISREG`, else refused; at most cap + 1 bytes read, and the file refused when the last byte arrives (§5.3: `layer.yaml` is a regular file, never a link or a folder). Adds rustix 1.1.6 or later with `fs` (decision-27). The walk is R3's. alz-io depends on alz-core for `SafeRelPath` and turns `ReadError` into a diagnostic.
- Binary: `crates/alz/src/command.rs` with `command/lint.rs`. `--layer <path>` lints that layer's `layer.yaml` (R2.5). Without it, a stack manifest in the working directory is linted as the manifest plus `layer.lock` when present (R2.6, R2.7); any other directory as a layer. Output as R2.1 states.
- R2 scope: lint checks only what R2 models. Instructions, skills, agents, settings, protected variables, the leak guard, the secret scan and slots arrive with R4 and R5 (R5 owns `lint`), and stack mode stops after a valid manifest and lock with a fixed `error [not-ported]` that names R3, exit 1 (owner answer 5); R3 removes it.

Tests: assert_cmd with files set up through alz-test-support; `layer.yaml` as a link, a folder, a FIFO, past 1 MiB and as invalid UTF-8, each refused without blocking.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 alz lint --layer <dir> exits 0 with an empty stderr for a valid layer.yaml, and 1 with error [layer] lines for a schema error (assert_cmd)
- [x] #2 A layer.yaml that is a link, a folder or a FIFO is refused without blocking, and one past 1 MiB is refused after reading at most 1 MiB + 1 byte (alz-io tests)
- [x] #3 alz lint in a directory with a stack manifest reports manifest and lock errors, and with a valid manifest and lock exits 1 with error [not-ported] naming R3 (assert_cmd; owner answer 5)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation notes (implementer). Files: Cargo.toml (workspace deps rustix 1.1.6 [std, fs] per decision-27, alz-io, alz-test-support); alz-io: fs.rs, fs/read_only_dir.rs (ReadOnlyDir::open, read_file: openat O_RDONLY|O_NONBLOCK|O_CLOEXEC|O_NOFOLLOW, fstat on the handle must be S_ISREG, take(cap+1) and refuse when the last byte arrives), fs/read_error.rs (ReadError -> layer/yaml/lock diagnostics); alz-test-support: ScratchDir (std only, mkfifo via the tool, no tempfile dependency); alz-core: lint.rs (pure: parse_document, is_stack_manifest, layer, stack_manifest with LockFile {Absent, Bytes, Unreadable}) and Code::NotPorted (temporary, not in spec); alz: cli.rs (clap derive Lint --layer), command/lint.rs, main.rs, report.rs (dead_code expect removed), tests/lint.rs (23 assert_cmd cases). AC1: tests valid_layer_lints_with_empty_stderr_and_summary, every_schema_error_of_a_layer_is_reported_and_none_prints_a_value (exit 1, error [layer] lines). AC2: alz-io tests for link, link to FIFO, folder, FIFO (each on a thread with a 10 s timeout), exact cap and cap+1, 3 MiB file, and read_at_most with an endless source serving exactly cap+1 bytes; alz tests repeat link/folder/FIFO/over 1 MiB/invalid UTF-8 through the binary. AC3: tests a_valid_manifest_and_lock_stop_at_not_ported_naming_r3, manifest_errors_are_reported_together_without_not_ported, a_manifest_error_and_a_lock_error_are_both_reported, lock link/FIFO/oversize. Design points: no --layer: layer.yaml with a stack field = manifest, anything else = layer (as the reference); stack mode prints diagnostics only, no summary line (R3 decides names); not-ported is added only when manifest and lock are otherwise clean. Acceptance against target/release/alz, 29 -t schema: 6 pass (the accepts cases), 21 fail. 20 fail only because the assertion expects the quoted "'account" and spec 10.1 prints a spec-defined field unquoted (output is 'error [layer] ../personal:layer.yaml:5:10: account must be ...'); with that assertion relaxed in a throwaway copy all 20 pass. The 21st (is a schema error for the build too) runs alz build, which is R5 (usage error exit 2). Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, msrv, deny, audit); mise run lint exit 0; mise run test 46 files, 1081 passed, 4 expected fail, 1 skipped. Open: R2.11 must adjust the quoted-account assertion and the build case; no spec text change was needed (spec 5.1/10.1 already describe the messages).

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (335 tests), mise run lint passes, src/ and test/ unchanged; the release binary reports an unknown field as one 'error [layer] ...:layer.yaml:4:1: unknown field ...' line plus 'lint failed: 1 error in ...', exit 1. Against test 29 'schema': 6 pass, 20 fail only on the reference's quoted "'account" wording (R2.11 asserts the spec form), 1 needs alz build (R5). Lead: stack mode prints no summary line until R3 (accepted); ScratchDir instead of tempfile (no new dependency).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The Rust binary has its first command: alz lint, for a layer (--layer or a layer.yaml without stack) and a stack manifest with its lock. layer.yaml and layer.lock are read through alz-io's no-follow, non-blocking, fstat-checked, capped read (rustix, decision-27): links, folders, FIFOs, files over 1 MiB and invalid UTF-8 are refused with diagnostics. Every schema error is reported at once and never prints a value. A valid stack stops with a temporary error [not-ported] until R3.
<!-- SECTION:FINAL_SUMMARY:END -->
