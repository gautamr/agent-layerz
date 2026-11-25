---
id: TASK-106
title: Replace the closed TargetName enum with the open TargetId
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:05:00Z'
updated_date: '2026-11-25T19:52:00Z'
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
- [ ] #1 grep -rn 'enum TargetName' crates finds nothing
- [ ] #2 targets: [antigravity] fails with the error 'target' at layer.yaml and the message no adapter for 'antigravity' (available: claude); targets: [codex] keeps its message and exit code (CLI tests)
- [ ] #3 A layer with antigravity sections in settings.yaml and models.yaml and a targets/antigravity/ folder builds for targets: [claude] with no diagnostic and writes none of them (test)
- [ ] #4 crates/alz-io/src/account uses its own account-family type and not TargetId (grep)
- [ ] #5 The acceptance suite, mise run rust:check and mise run alz:check pass with unchanged output
<!-- AC:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-25T19:52:00Z

---

From TASK-104 (spec, @architect): §5.2 'Target ids' states what this task makes true. Beyond AC #2 and #3, the spec also says: the passthrough error for a folder of targets/ that is not a target id gets the message 'this file is below a folder of targets/ that is not a target id (§5.2)' (§7.11); the account map (§7.13) and alz use keep claude and codex only; 'no adapter for' lists the registered ids in name order joined with ', ' and each such target is reported once (§8 'Targets without an adapter'). Agent native sections (§7.2) are left closed in the spec pending an owner question. docs/spec-coverage.md rows §5.2 Target ids and §8 Targets without an adapter are planned with this task.

---
<!-- COMMENTS:END -->
