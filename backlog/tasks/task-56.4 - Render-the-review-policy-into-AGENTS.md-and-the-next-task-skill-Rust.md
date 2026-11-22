---
id: TASK-56.4
title: Render the review policy into AGENTS.md and the next-task skill (Rust)
status: To Do
assignee: []
created_date: '2026-11-22T13:49:00Z'
updated_date: '2026-11-22T13:50:00Z'
labels:
  - area:process
  - area:cli
milestone: m-3
dependencies:
  - TASK-56.3
parent_task_id: TASK-56
priority: medium
ordinal: 46390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Render the merged policy into generated output: a short Review section in AGENTS.md (when reviews run, which reviewers under which conditions, what they gate) and the review slot of the next-task skill, replacing the fixed sentence in step 5. Output is deterministic and byte-identical for the same inputs; a repository with no declaration gets wording equivalent to today's rule.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 For unit: subtask, task, parent and milestone, the rendered AGENTS.md section and next-task skill state the cadence exactly, and alz check fails when they are edited by hand
- [ ] #2 Building this repository with no review declaration produces a Review section equivalent to today's step 5, with no other change to generated files
<!-- AC:END -->
