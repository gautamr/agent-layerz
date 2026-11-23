---
id: TASK-55.5.4
title: 'R4.4: Instructions and path-scoped rules'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
labels:
  - area:rust
  - area:merge
milestone: m-7
dependencies:
  - TASK-55.5.3
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 117390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step.
- item/instruction.rs and merge/instructions.rs (§7.1): blocks in stack order, then by file name in code-point order (§6), each tagged with its layer's kind and name. `mode: replace` and `mode: remove` in a more specific layer act on the more general block with the same file name. Unknown fields and other modes are `instructions` errors. targets/claude/CLAUDE.extra.md is collected per layer for R5. The instructions_max_lines budget counts the composed AGENTS.md and stays with R5's render.
- item/rule.rs and merge/rules.rs (§7.10): identity is the kebab-case file name; the most specific layer wins with the whole file; `mode: extend` appends its body under a layer heading and unions `paths`; paths entries are non-empty globs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 02's case has a Rust test with the same verdict: three blocks in stack order, mode: remove drops a block and mode: replace replaces one (unit tests)
- [ ] #2 Test 17's merge cases have Rust tests with the same verdict: a repo rule replaces the personal one, and an extend rule appends a section and unions paths (unit tests)
- [ ] #3 Proptest: the rule merge is associative, path unions are idempotent, and block order depends only on stack order and code-point name order
<!-- AC:END -->
