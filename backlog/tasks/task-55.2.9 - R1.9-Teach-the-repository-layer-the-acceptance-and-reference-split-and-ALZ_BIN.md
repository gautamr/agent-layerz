---
id: TASK-55.2.9
title: >-
  R1.9: Teach the repository layer the acceptance and reference split and
  ALZ_BIN
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-22T17:57:00Z'
updated_date: '2026-11-23T03:04:00Z'
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
- [x] #1 The owner approved the text in a comment on this task before it was applied
- [x] #2 `mise run alz:build` ran and `mise run alz:check` passes; the generated AGENTS.md and the add-acceptance-test skill describe ALZ_BIN, test/reference and the import guard
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-22): layer text change approved.

Applied the owner-approved text (2026-11-22) in .layer/ only: skills/add-acceptance-test/SKILL.md (binary under test, ALZ_BIN, test:acceptance, spawn helpers, tempDir, import guard, decision-26 assertion form, test/reference and test/spec), rules/tests.md (same rules, short), instructions/50-repo.md (Commands: test:acceptance with ALZ_BIN; Map: acceptance, reference, spec, guards), agents/implementer.md (step 3 and 5). Every path, command and helper named was checked to exist. Checks: mise run alz:build (68 files, 4 updated), mise run alz:check (passed), mise run lint (biome and tsc clean). git diff --stat: only .layer/, generated .claude/ and AGENTS.md and layer.lock, and this task file changed. Not committed; ACs left unchecked.

Verified by the lead (2026-11-23): `mise run alz:check` passes (68 files; the two agent-replace warnings predate this task); diff limited to the four .layer/ files and their generated outputs.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The repository layer now describes the language-neutral suite: the acceptance project spawns the binary under test (ALZ_BIN, else the TypeScript reference), never imports src/ (guarded), takes temp folders from tempDir(), and asserts help, usage and version in the decision-26 form; TypeScript-only cases live in test/reference/ until cutover. Covers the add-acceptance-test skill, the tests rule, the repo instructions and the implementer agent; generated files rebuilt with mise run alz:build.
<!-- SECTION:FINAL_SUMMARY:END -->
