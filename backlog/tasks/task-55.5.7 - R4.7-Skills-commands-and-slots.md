---
id: TASK-55.5.7
title: 'R4.7: Skills, commands and slots'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T13:38:00Z'
labels:
  - area:rust
  - area:merge
milestone: m-7
dependencies:
  - TASK-55.5.6
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 120390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.3, §7.4).
- item/skill.rs: a skill is its directory with all its files from LayerContents; the SKILL.md name equals the directory name (kebab-case) and a description is required. commands/<name>.md becomes a skill that only explicit invocation reaches; a command and a skill with one name in one layer is an error.
- merge: Replace per name, the most specific layer wins with the whole directory.
- Slots: every name in a merged agent's skills or in a layer's requires_slots must exist; a provides_slots entry must name a skill of that layer. strict_slots true gives a `slot-missing` error; false gives a stub skill and a warning. The user-directory collision report is doctor's (R5).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 05's merge verdict has a Rust test: the repo skill replaces the personal one with its whole directory (unit test)
- [ ] #2 Test 06's cases have Rust tests with the same verdict: a missing slot fails with strict_slots true and gives a stub and a warning with false (unit tests)
- [ ] #3 A SKILL.md name that differs from its directory, a command and a skill sharing a name in one layer, and a provides_slots entry without its skill are errors (unit tests)
<!-- AC:END -->
