---
id: TASK-55.7.2
title: 'R6.2: Spec for the identity steps, account which and allow, and the R6 codes'
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T08:20:00Z'
labels:
  - area:rust
  - area:docs
  - needs:owner-approval
milestone: m-7
dependencies: []
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 161390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (like R5.1). Write into the spec every line the acceptance suite asserts for R6, with its stream, level and code, so the Rust messages follow the spec (decision-26):
- §9.4 and §10.3: the identity steps of init, build, build --adopt and check (note outside git, warning or error per command, the finding lines, the fix commands, the `git init` and top-level texts, the written-key line).
- §10.3: `alz account which` (repository line, one line per target with its source, each skipped step, the two file paths, stale approvals, fails-closed lines and fixes), `alz account allow` and `--remove` (offer, prompts, `already allowed`, `nothing to remove`, `not confirmed; nothing changed`, refusals), and the `note [account-allow]` forms.
- §10.1 codes: identity as warning and note, account-config, account-allowed, account-allow, account at resolution.
- The owner answers to TASK-55.7's R6 questions as spec text (the .git/config write race, the identity environment, the home lookup, Ctrl-C at the prompt, ALZ_ACCOUNT_DIRS, the blocked folders).
- §7.13 and §9.1 no longer say these parts are built in TypeScript only; §13 entries and the drift list for R6.
- CHANGELOG entry: no generated file changes; allowed.yaml readable for every root; stricter refusals.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Spec §9.4, §10.1 and §10.3 state every identity, account which, account allow and hint line that tests 24, 26 and 28 to 32 assert, with stream, level and code (spec diff; a grep of each asserted string finds it in the spec)
- [ ] #2 The owner's answers to the R6 questions in TASK-55.7 are spec text, and §13's drift list names every R6 reference-drift case (spec diff)
- [ ] #3 CHANGELOG has the user-facing R6 entry and its version note (diff)
<!-- AC:END -->
