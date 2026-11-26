---
id: TASK-111
title: Move the Claude adapter into the alz-target-claude crate
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
updated_date: '2026-11-26T08:18:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-109
  - TASK-110
references:
  - decision-40
priority: medium
ordinal: 212390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-40 point 6: a built-in adapter that depends only on alz-adapter cannot see core internals or do I/O, by the compiler. This is the move, with byte-identical output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 crates/alz-target-claude depends on alz-adapter and on neither alz-core nor alz-io (Cargo.toml grep), with the clippy.toml bans
- [ ] #2 crates/alz-core/src/target/claude no longer exists (ls)
- [ ] #3 The acceptance suite, mise run rust:ci and mise run alz:check pass with unchanged output
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (commit 3358a14 on feat/claude-crate-task-111). AC1: crates/alz-target-claude/Cargo.toml [dependencies] holds alz-adapter only (dev: proptest); no alz-core, no alz-io; clippy.toml is a twin of alz-adapter's bans, and the twin headers of all five clippy.toml files name each other; #![forbid(unsafe_code)]. AC2: crates/alz-core/src/target/claude no longer exists (ls: no such file). Moved into alz-adapter as contract helpers: Stripped, QuotedKey, is_control, is_format and the Cf table with its UCD fixture test (alz-core re-exports them, so its own tests and the matrix paths for untrusted_text.rs stay). The adapter's neutral_tool and neutral_permission_mode now answer in contract words (&str, Mode); alz-core maps Mode to PermissionMode (adapter_model::neutral_mode). The doctor's Claude Version and VERIFIED_RANGE (regex-lite, restricted to alz-core by decision-38) moved to alz-core/src/doctor/claude_version.rs. Registry::builtin() stays in alz-core, which now depends on alz-target-claude (no cycle: the adapter crate depends on alz-adapter only; adoption and doctor in the core already know Claude). Stack-level tests (golden fixtures, facts, settings) need the merge, so they live in alz-core/src/adapter_model/claude_stacks*; the new crate has 6 own tests. AC3: mise run rust:ci exit 0 (fmt, clippy -D warnings, all tests incl. acceptance and spec_coverage guard, doc, msrv, deny 'advisories ok, bans ok, licenses ok, sources ok', audit); mise run alz:check: 'check passed: 67 generated files match the layers and the lock'. docs/LAYERING-SPEC.md names none of the moved paths, so no spec change; docs/spec-coverage.md paths updated.
<!-- SECTION:NOTES:END -->
