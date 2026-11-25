---
id: TASK-111
title: Move the Claude adapter into the alz-target-claude crate
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
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
