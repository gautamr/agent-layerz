---
id: TASK-55.6.10
title: 'R5.10: alz ls and alz explain'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T03:42:00Z'
labels:
  - area:rust
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.6.8
  - TASK-55.6.17
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 142390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10), from Merged's provenance and the Rendered tree.
- listing.rs: the sections agents, skills, commands, rules, hooks and mcp (enabled servers), each with its count and, per item, the name padded as the reference pads it and the layer that decides it.
- explain.rs: types agent, skill, command, rule, hook, mcp and instruction; the sources general to specific with their actions; a defined but not enabled server says so; then the merged output file or files from the tree. An unknown type is a usage error (exit 2); an unknown name exits 1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz ls on fixtures 01 and 22 prints the sections, counts and deciding layers the acceptance tests assert (vitest run of test 22 without --live)
- [ ] #2 alz explain for each type lists the sources in stack order and the merged file, and the explain cases of tests 03 and 17 pass against ALZ_BIN (vitest run)
- [ ] #3 Untrusted names and actions are printed only as validated names or stripped text (unit tests)
<!-- AC:END -->
