---
id: TASK-97
title: >-
  Record the facts behind 11 Claude output keys that target-facts does not
  record
status: To Do
assignee: []
created_date: '2026-11-25T14:45:00Z'
labels:
  - bug
  - area:docs
dependencies: []
ordinal: 199390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Spec §8 says an adapter relies only on what docs/target-facts.md records. A test (target::claude::tests::facts, task-55.9.7) reads every key the Claude adapter writes and finds 11 that no verified row records: skill frontmatter name, description and argument-hint (§5 records only disable-model-invocation); permissions allow, ask and deny (§7b gives the rule syntax and order, but names none of the three keys); and the keys of a .mcp.json server: command, args, env, type, url (§7 and §13.4 say only that servers live in .mcp.json; the shape is recorded for Codex only). Verify each with the verify-target-fact skill (DOCS at least, SMOKE where possible), record them in target-facts, and shrink NO_FACT_RECORDS in crates/alz-core/src/target/claude/tests/facts.rs to empty, then mark the spec-coverage row for §8 What an adapter may rely on covered. Do not change what alz writes: these keys are the ones Claude reads today.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each of the 11 keys has a verified row in docs/target-facts.md (DOCS at least)
- [ ] #2 NO_FACT_RECORDS in the facts test is empty and the test passes
- [ ] #3 The spec-coverage row for §8 What an adapter may rely on is covered
<!-- AC:END -->
