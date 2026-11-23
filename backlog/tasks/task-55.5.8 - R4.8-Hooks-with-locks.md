---
id: TASK-55.5.8
title: 'R4.8: Hooks with locks'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
labels:
  - area:rust
  - area:merge
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.7
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 121390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.5).
- item/hook.rs: hooks.yaml is a list of HookEntry = Define(Hook) | Disable(HookId), a disable holding only id and disabled: true. Hook { id, event: Neutral(NeutralEvent) | Native { target, name } with its matcher, match (tools, paths), action: Run(SafeRelPath inside the layer) | Command(CommandLine, ${PROJECT_DIR} kept for the adapter), on_fail, targets, timeout in seconds, locked }.
- merge: every hook of every layer, in stack order then file order. An id that a more general layer already defines is an error. A disable removes an unlocked hook of a more general layer; disabling a locked hook is an error naming the layer; disabling an unknown id is `unresolved`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 08's cases have Rust tests with the same verdict: disabling a locked hook fails naming the layer, and disabling an unlocked hook removes it (unit tests)
- [ ] #2 Test 19's cases have Rust tests with the same verdict: command hooks are kept as written with ${PROJECT_DIR}, native events keep their matcher, and a hook with both run and command is refused (unit tests)
- [ ] #3 Proptest: for any stack and any further layer the merge refuses or keeps every locked hook enabled, and hook order is stack order then file order
<!-- AC:END -->
