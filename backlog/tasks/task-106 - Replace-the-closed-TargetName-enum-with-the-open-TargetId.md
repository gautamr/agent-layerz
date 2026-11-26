---
id: TASK-106
title: Replace the closed TargetName enum with the open TargetId
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-25T19:05:00Z'
updated_date: '2026-11-26T05:29:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-105
references:
  - decision-40
priority: medium
ordinal: 207390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TargetName {Claude, Codex} is matched in about 40 files; decision-40 point 5 opens it. Account-folder families (.claude, .codex) are not adapters and get their own closed type. Output must not change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 grep -rn 'enum TargetName' crates finds nothing
- [x] #2 targets: [antigravity] fails with the error 'target' at layer.yaml and the message no adapter for 'antigravity' (available: claude); targets: [codex] keeps its message and exit code (CLI tests)
- [x] #3 A layer with antigravity sections in settings.yaml and models.yaml and a targets/antigravity/ folder builds for targets: [claude] with no diagnostic and writes none of them (test)
- [x] #4 crates/alz-io/src/account uses its own account-family type and not TargetId (grep)
- [x] #5 The acceptance suite, mise run rust:check and mise run alz:check pass with unchanged output
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (commits b9e06b0, 932f67d on feat/open-target-ids-task-106). AC1: grep -rn 'enum TargetName' crates finds nothing; alz-core names targets through alz_adapter::TargetId, with crate::target::claude()/codex() as the two ids the code names and REGISTERED = [claude] for the 'available' list. AC2: crates/alz/tests/open_target_ids.rs a_target_id_without_an_adapter_is_the_error_target_listing_the_available_adapters (exit 1, 'error [target] layer.yaml: no adapter for antigravity (available: claude)', tree unchanged), each_target_without_an_adapter_is_reported_once, the_codex_target_keeps_its_message_and_exit_code (same message, exit 1). AC3: sections_for_an_id_that_is_not_built_for_give_no_diagnostic_and_are_never_written (antigravity in settings.yaml, models.yaml, targets/antigravity/: exit 0, empty stderr, same generated files as a build without them, none holds their content). This test exposed that the raw passthrough warning was emitted at read time for a file never written; the warning now waits in the merge state and finish keeps only those of targets in the manifest (lint of a layer alone still warns for all). AC4: crates/alz-io/src/account has no TargetId; the closed AccountFamily {Claude, Codex} is defined in alz-core/src/account/family.rs (the account model is in alz-core) and used by alz-core account, alz-io account and alz account/use code. AC5: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests incl. golden-trees and fixture tests unchanged, doc, MSRV, deny, audit); mise run alz:check: 'check passed: 67 generated files match the layers and the lock'. Existing tests that used lower-case words as unknown keys or targets (bogus, gemini, cursor, other, constructor...) now use non-id spellings, since those words are target ids. Spec: §5.2/§7.9/§7.11/§8 text updated (no warning for files of an id not built for); docs/spec-coverage.md rows 5.2 Target ids and 8 Targets without an adapter are covered and the summary updated. Left open: agent native sections (7.2) and hook native events stay closed to claude/codex; native key tables still keep target as a static str until TASK-110.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-25T19:52:00Z

---

From TASK-104 (spec, @architect): §5.2 'Target ids' states what this task makes true. Beyond AC #2 and #3, the spec also says: the passthrough error for a folder of targets/ that is not a target id gets the message 'this file is below a folder of targets/ that is not a target id (§5.2)' (§7.11); the account map (§7.13) and alz use keep claude and codex only; 'no adapter for' lists the registered ids in name order joined with ', ' and each such target is reported once (§8 'Targets without an adapter'). Agent native sections (§7.2) are left closed in the spec pending an owner question. docs/spec-coverage.md rows §5.2 Target ids and §8 Targets without an adapter are planned with this task.

---
<!-- COMMENTS:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Targets are open ids (TargetId of alz-adapter) everywhere in the core; an id without an adapter is 'no adapter for', codex keeps its message; sections of an id not built for are checked and never written; accounts use their own AccountFamily. Output byte-identical. Owner approved the spec wording; merged in #41.
<!-- SECTION:FINAL_SUMMARY:END -->
