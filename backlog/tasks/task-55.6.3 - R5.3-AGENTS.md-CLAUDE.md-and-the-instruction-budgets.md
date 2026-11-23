---
id: TASK-55.6.3
title: 'R5.3: AGENTS.md, CLAUDE.md and the instruction budgets'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T18:08:00Z'
labels:
  - area:rust
  - area:render
milestone: m-7
dependencies:
  - TASK-55.6.2
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 135390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.1, §7.12, §8.1).
- AGENTS.md from Merged::instructions: the header with every block's source ref (and the declaring layer's layer.yaml when a commit identity is declared; `no instructions` when there is none), one `## <Kind>: <name>` heading per layer run (kind with its first character in upper case), the blocks, then the §7.12 Commits section from the winner of Merged::commit_identity.
- The budget check of the composed text, Commits section included: instructions_max_lines (lines counted as the reference does) and instructions_max_bytes, each an error `budget`.
- CLAUDE.md: header naming AGENTS.md and each CLAUDE.extra.md, `@AGENTS.md`, then `## Claude-specific` and the extras in stack order.
Byte parity with the reference for fixtures 01, 02, 16 and 26.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 AGENTS.md and CLAUDE.md match the reference's bytes for fixtures 01, 02, 16 and 26, Commits section included (Rust tests over Merged values built from those fixtures)
- [ ] #2 A composed AGENTS.md over instructions_max_lines or instructions_max_bytes is the error budget, and the Commits section counts toward both (unit tests at the limit and one over it)
- [ ] #3 Rendering is deterministic: render(m) == render(m) byte for byte for generated stacks (proptest)
<!-- AC:END -->
