---
id: TASK-7
title: Write CONTRIBUTING.md
status: In Progress
assignee:
  - '@claude'
created_date: '2026-11-19T17:13:00Z'
updated_date: '2026-11-28T02:58:00Z'
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

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Write CONTRIBUTING.md: setup through mise; using and building at the same time (stable alz from install:local, a working copy as alz-dev, cargo run); the checks and what CI runs; dogfooding (generated files come from .layer/); the code map; spec, tests (spec-test skill, fixtures/) and target facts; the board, one task per PR, Conventional Commits, the commit identity, releases from CI only.
2. Check every command in it by running it or reading mise.toml.
3. Fresh-context review.
<!-- SECTION:PLAN:END -->
