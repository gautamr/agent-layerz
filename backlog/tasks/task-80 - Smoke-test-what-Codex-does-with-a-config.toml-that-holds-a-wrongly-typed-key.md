---
id: TASK-80
title: Smoke-test what Codex does with a config.toml that holds a wrongly typed key
status: Done
assignee:
  - '@docs-writer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-23T23:32:00Z'
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
- [x] #1 The behaviour is recorded in target-facts at SMOKE level with the command used
- [x] #2 A follow-up task exists if Codex drops more than the one key
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Done in PR #11 (2026-11-24): target-facts §6.7 records at SMOKE level (Codex 0.169.0, temporary CODEX_HOME) that one wrongly typed key or unknown enum value makes Codex refuse the whole config.toml, user and trusted project alike; unknown keys load. Follow-up TASK-82 created.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Codex refuses its whole config.toml over one wrongly typed key; recorded in target-facts §6.7; TASK-82 carries the rule for the Codex adapter. Merged as PR #11.
<!-- SECTION:FINAL_SUMMARY:END -->
