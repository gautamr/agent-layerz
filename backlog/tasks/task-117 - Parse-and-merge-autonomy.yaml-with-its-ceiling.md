---
id: TASK-117
title: Parse and merge autonomy.yaml with its ceiling
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:07:00Z'
labels:
  - area:autonomy
milestone: m-8
dependencies:
  - TASK-116
priority: medium
ordinal: 218390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The neutral half of doc-7 §6 and §7: one item file per layer, level last-writer-wins under a ceiling no more specific layer can raise, never and fixed classes absorbing. No adapter output yet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An unknown key, an unknown level or a row other than alone, notify, ask or never is the error autonomy naming the file and key (tests)
- [ ] #2 Proptest laws: the effective level is at most every ceiling; a never class stays never; a fixed class stays ask; a higher level never makes a class's row stricter
- [ ] #3 A level above an inherited ceiling, or a ceiling raised by a more specific layer, is the error autonomy-floor naming both layers (tests)
- [ ] #4 A stack without autonomy.yaml renders byte-identical output (acceptance suite and mise run alz:check)
<!-- AC:END -->
