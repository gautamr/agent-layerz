---
id: TASK-76
title: Record the security-relevant Claude and Codex settings keys in target-facts
status: To Do
assignee: []
created_date: '2026-11-23T16:08:00Z'
updated_date: '2026-11-23T18:05:00Z'
labels:
  - area:docs
  - area:security
milestone: m-7
dependencies: []
priority: high
ordinal: 131390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R4.16 enforces floors and reserved keys for settings keys most of which are not yet in docs/target-facts.md (the architect's draft table in TASK-55.5's notes marks them UNVERIFIED, read from Claude Code 2.1.323's settings reference and Codex 0.169.0's config schema). Verify each key's name, type and effect with the verify-target-fact skill and record it, at DOCS level at least, before R5's adapter relies on any of them; correct R4.16's list where a key turns out wrong.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every key of the approved table has a target-facts entry with its source and level
- [ ] #2 Any key that turns out wrong or renamed is fixed in the R4.16 list and the spec
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Recorded all 98 rows of crates/alz-core/src/item/native_keys.rs in docs/target-facts.md section 13: DOCS for every row, SMOKE (type and enum values, through a temporary CLAUDE_CONFIG_DIR and CODEX_HOME) for 94. Corrections for the table are listed in 13.8 for AC #2 (implementer follow-up): codex tools.web_search is bool-or-table, not a mode; allowAllClaudeAiMcps is managed only; plus missing keys.
<!-- SECTION:NOTES:END -->
