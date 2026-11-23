---
id: TASK-55.5.16
title: >-
  R4.16: Security-relevant native settings as floors and reserved keys,
  permission floor fields, no servers or hooks in native fields
status: To Do
assignee: []
created_date: '2026-11-23T16:08:00Z'
labels:
  - area:rust
  - area:security
  - area:merge
milestone: m-7
dependencies:
  - TASK-55.5.15
parent_task_id: TASK-55.5
priority: high
ordinal: 130390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner decisions of 2026-11-23 (TASK-55.5's notes, the architect's approved draft table). In alz-core: (a) a fixed list of security-relevant native settings keys per target with floor semantics — once a layer sets one, a more specific layer may only move it up its stricter order (booleans, ordered enums, 'no additions' lists, 'union' lists); a looser value or a kind change is a settings error naming the layer, file and key path; (b) the reserved keys of the table are settings errors wherever they appear (top level and nested as the table says); (c) permissions.yaml gains first-class floor fields for disableBypassPermissionsMode and blocking reads and writes outside the working directories (once true, stays true), rendered by R5; (d) claude.mcpServers and per-agent hooks in an agent's native fields are agent errors. Each with unit and property tests (a floor key never loosens down the stack), and neutral acceptance rows in test 37 through lint. The architect writes the spec text (§7.6, §7.9, §7.2) with reference drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every floor key of the approved table is refused when a more specific layer loosens it or changes its kind, with a settings error naming the layer, file and key path, and a property test shows no floor key loosens down the stack
- [ ] #2 Every reserved key of the table is refused in settings.yaml, and claude.mcpServers and per-agent hooks are refused in an agent's native fields
- [ ] #3 permissions.yaml accepts the new floor fields (once true, stays true), and the spec states every rule of this task with its reference drift
<!-- AC:END -->
