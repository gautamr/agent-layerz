---
id: TASK-7
title: Write CONTRIBUTING.md
status: To Do
assignee:
  - '@docs-writer'
created_date: '2026-11-19T17:13:00Z'
labels:
  - area:docs
milestone: m-1
dependencies: []
priority: medium
ordinal: 200
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Contributors need the setup, the test conventions and the rules this project holds itself to, including that its own agent configuration is generated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Covers setup with mise and the test, lint and build commands
- [ ] #2 Explains acceptance tests and fixtures, pointing to the add-acceptance-test skill
- [ ] #3 States Conventional Commits with the task ID and one task per pull request
- [ ] #4 States that adapter behaviour must rest on a recorded fact in docs/target-facts.md
- [ ] #5 Explains that AGENTS.md, CLAUDE.md and .claude/ are generated from .layer/ with `mise run alz:build`
<!-- AC:END -->
