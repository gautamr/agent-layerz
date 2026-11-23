---
id: TASK-76
title: Record the security-relevant Claude and Codex settings keys in target-facts
status: To Do
assignee: []
created_date: '2026-11-23T16:08:00Z'
updated_date: '2026-11-23T18:17:00Z'
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
- [x] #2 Any key that turns out wrong or renamed is fixed in the R4.16 list and the spec
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Recorded all 98 rows of crates/alz-core/src/item/native_keys.rs in docs/target-facts.md section 13: DOCS for every row, SMOKE (type and enum values, through a temporary CLAUDE_CONFIG_DIR and CODEX_HOME) for 94. Corrections for the table are listed in 13.8 for AC #2 (implementer follow-up): codex tools.web_search is bool-or-table, not a mode; allowAllClaudeAiMcps is managed only; plus missing keys.

AC #2 done: corrections 1 to 7 of docs/target-facts.md 13.8 are applied in crates/alz-core/src/item/native_keys.rs and docs/LAYERING-SPEC.md 7.9 (and the R5 disable string in 7.6). Codex tools.web_search is no longer a ladder (only tools.web_search.allowed_domains is a no-additions floor); allowAllClaudeAiMcps is OUTSIDE_THE_PROJECT; claude.mcpServers is reserved with a reason that does not call it a settings key; codex profile is reserved as removed by 0.169.0; codex profiles and windows are reserved whole (the profiles.* first-class and floor rows, unreachable behind the whole-key refusal, are folded); 4 command keys and 11 managed or user keys added. Evidence: mise run rust:ci green (988 alz-core tests); mise run lint clean; acceptance 37 against the release binary: all lint rows pass, 22 failures are only the alz build cases that need R5 (TASK-55.6).
<!-- SECTION:NOTES:END -->
