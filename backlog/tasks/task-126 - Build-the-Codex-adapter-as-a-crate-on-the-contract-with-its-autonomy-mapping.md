---
id: TASK-126
title: Build the Codex adapter as a crate on the contract, with its autonomy mapping
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:08:00Z'
updated_date: '2026-11-26T03:23:00Z'
labels:
  - area:adapter
  - area:autonomy
milestone: m-8
dependencies:
  - TASK-112
  - TASK-115
  - TASK-119
  - TASK-127
references:
  - TASK-18
  - decision-40
priority: medium
ordinal: 227390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-18 is the Codex adapter; decision-40 has it built as alz-target-codex on the contract, and doc-7 §10 adds the autonomy mapping. This task carries TASK-18's acceptance criteria onto the contract; the owner decides whether TASK-18 becomes its parent or moves to this milestone.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 crates/alz-target-codex depends only on alz-adapter (Cargo.toml grep) and meets TASK-18's acceptance criteria #2 to #5 (acceptance test 13 and doctor test)
- [ ] #2 A golden .codex/config.toml per level holds approval_policy and sandbox_mode as doc-7 §10 maps them, and no level writes danger-full-access (tests)
- [ ] #3 The facts harness passes for Codex against its facts file (test)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-26): decision-40 accepted. When an adapter cannot express a security item (for example a decision class's rules on a target without allow/ask/deny lists), alz fails the build with an error naming the item and the target, never a warning; a layer may opt that target out of the item explicitly.
<!-- SECTION:NOTES:END -->
