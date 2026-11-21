---
id: TASK-29
title: Make use-case traceability the owner's standard across projects
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T18:08:00Z'
labels:
  - area:process
  - needs:owner-approval
milestone: m-3
dependencies:
  - TASK-19
  - TASK-28.13
priority: medium
ordinal: 315
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Once the pilot has proven itself on Agent Lens, every project should inherit the same standard from the owner's personal layer instead of copying it. This is also the first real use of alz promote. The personal layer is shared by every project, so the owner approves the change first.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The owner approves the change to the personal layer before it is made
- [ ] #2 The use-cases skill and the uc:matrix and uc:check tooling are promoted from this repository's layer to the personal layer with alz promote, and the personal layer is released with a new tag
- [ ] #3 This repository stacks the new personal layer version and still passes uc:check
- [ ] #4 The standard is applied to the owner's private client hub projects (Spec Kit specs/NNN-slice/spec.md keep their format and gain the same IDs and labels); no private names are recorded in this public repository
<!-- AC:END -->
