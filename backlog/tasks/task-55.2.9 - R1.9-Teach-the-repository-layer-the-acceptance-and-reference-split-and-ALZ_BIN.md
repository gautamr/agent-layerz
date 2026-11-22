---
id: TASK-55.2.9
title: >-
  R1.9: Teach the repository layer the acceptance and reference split and
  ALZ_BIN
status: To Do
assignee: []
created_date: '2026-11-22T17:57:00Z'
updated_date: '2026-11-22T18:02:00Z'
labels:
  - area:agents
  - area:test
milestone: m-7
dependencies:
  - TASK-55.2.7
references:
  - .layer/skills/add-acceptance-test/SKILL.md
  - .layer/rules/tests.md
  - .layer/instructions/50-repo.md
  - .layer/agents/implementer.md
parent_task_id: TASK-55.2
priority: medium
type: task
ordinal: 79390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
After R1.7, the instructions that agents load still describe the old in-process suite. This is a change to the agent layer, so the owner approves it (delivery-protocol §4).

Proposed text changes:
- .layer/skills/add-acceptance-test/SKILL.md and .layer/rules/tests.md:
  - acceptance tests never import src/ (the guard in test/guards enforces it) and drive the binary only through test/helpers.ts;
  - temp folders come from `tempDir()`;
  - assertions follow the spec, and help, usage and version follow decision-26 (exit code, stream, contains the version);
  - TypeScript-only cases go to test/reference/ and are deleted at cutover.
- .layer/instructions/50-repo.md (Commands and Map): `mise run test:acceptance`, ALZ_BIN, test/reference/.
- .layer/agents/implementer.md: wherever it describes the suite.

Then `mise run alz:build` and `mise run alz:check`. Generated files are never edited by hand.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The owner approved the text in a comment on this task before it was applied
- [ ] #2 `mise run alz:build` ran and `mise run alz:check` passes; the generated AGENTS.md and the add-acceptance-test skill describe ALZ_BIN, test/reference and the import guard
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-22): layer text change approved.
<!-- SECTION:NOTES:END -->
