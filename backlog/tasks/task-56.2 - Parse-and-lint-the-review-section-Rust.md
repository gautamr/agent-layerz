---
id: TASK-56.2
title: Parse and lint the review section (Rust)
status: To Do
assignee: []
created_date: '2026-11-22T13:49:00Z'
updated_date: '2026-11-22T13:50:00Z'
labels:
  - area:process
  - area:cli
milestone: m-3
dependencies:
  - TASK-56.1
  - TASK-55.9
parent_task_id: TASK-56
priority: medium
ordinal: 44390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Model the review section as validated Rust types (an enum for unit and gate, a newtype for agent names, typed conditions), parsed once (parse, don't validate) with errors that name the layer and field and never repeat a rejected value. alz lint reports unknown values, a reviewer that no layer defines as an agent, and an empty when. Follows the Rust standard in doc-5.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz lint accepts every example in the spec section and rejects each invalid form with a message naming the layer and field, never the value
- [ ] #2 Unit tests and proptest cover the parser; an acceptance test covers lint output
<!-- AC:END -->
