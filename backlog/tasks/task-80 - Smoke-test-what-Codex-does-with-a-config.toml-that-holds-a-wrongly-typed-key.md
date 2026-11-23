---
id: TASK-80
title: Smoke-test what Codex does with a config.toml that holds a wrongly typed key
status: In Progress
assignee:
  - '@docs-writer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-23T18:17:00Z'
labels:
  - area:docs
  - area:security
milestone: m-7
dependencies: []
priority: high
ordinal: 150390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner (2026-11-23): before deciding the Codex rule, record in docs/target-facts.md what Codex 0.169.0 does when config.toml holds one key of the wrong type: whether it refuses the whole file, ignores that key, or something else, and whether project-local config behaves the same. Use a temporary CODEX_HOME only. If Codex drops more than the one key, the typed-allowlist rule of the Claude task applies to Codex too (file a follow-up).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The behaviour is recorded in target-facts at SMOKE level with the command used
- [ ] #2 A follow-up task exists if Codex drops more than the one key
<!-- AC:END -->
