---
id: TASK-28.12
title: Standardise use-case traceability (pilot in this repository)
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T18:08:00Z'
labels:
  - enabler
  - area:process
milestone: m-5
dependencies: []
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
parent_task_id: TASK-28
priority: high
ordinal: 85
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Use cases and tasks need a standard link that agents follow and people can see on the board. Backlog.md's board filters by label (F) and task lists filter by --labels, so a use-case label plus a document link gives both linkage and view. Piloted here before it becomes the owner's standard everywhere.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A use-cases skill in .layer/skills defines the standard: IDs UC-<prefix><n>; scenarios UC-<prefix><n>.S<m> in Given/When/Then; one spec per feature as a Backlog document of type specification (or specs/<feature>/spec.md in Spec Kit repositories)
- [ ] #2 The skill defines the task rules: a uc:<ID> label per use case served plus a document link to the spec; enabler for work that serves no use case; epic for epics; acceptance criteria and acceptance test names cite the scenario IDs they implement
- [ ] #3 mise run uc:matrix prints each use case with its tasks, status and criteria progress; uc:matrix --write refreshes a Traceability document visible in backlog browser
- [ ] #4 mise run uc:check fails on a task with no uc:, enabler or epic label, and on a use case in a specification document that no task serves
- [ ] #5 The architect preloads the use-cases skill, and /next-task refuses to plan a feature task that is not traced
- [ ] #6 The Agent Lens epic and subtasks follow the standard and uc:check passes; the uc:matrix output is recorded in the task notes
<!-- AC:END -->
