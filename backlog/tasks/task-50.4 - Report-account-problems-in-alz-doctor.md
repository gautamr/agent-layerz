---
id: TASK-50.4
title: Report account problems in alz doctor
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-22T01:37:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.2
parent_task_id: TASK-50
priority: medium
ordinal: 20390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Covers the account folder checks. Workspace trust lives in per-account state (.claude.json, Codex config.toml) that alz never opens, so alz never determines whether a repository is trusted. TASK-22 (per-account trust) folds into this work as a static notice in alz doctor that trust is per account (LAYERING-SPEC §7.13 'Trust is per account', §9.1 'names only').
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz doctor reports, per repository, a named account folder that does not exist and an account variable set outside alz (naming mise or direnv only where LAYERING-SPEC §12 item 20 confirms how to detect them); it ignores the blocked folders ~/.claude-alz-blocked and ~/.codex-alz-blocked
- [ ] #2 It reports the shared-by-default items (claude agents, commands, skills, plugins, CLAUDE.md; codex AGENTS.md, skills) that are not linked in an account folder, by name only, and does not flag settings.json, projects/, sessions/ or memories/, which stay per account on purpose
- [ ] #3 Every check is tested with a fake HOME and never opens a file inside an account folder
- [ ] #4 alz doctor prints a static notice that workspace trust is per account and never reads trust state (test: no file under the account folders or ~/.claude.json is opened)
- [ ] #5 alz doctor does not count ~/.claude-alz-blocked or ~/.codex-alz-blocked as account folders, but warns when one exists (a tool may create it at a blocked start)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (@architect): revised to the owner's shared-items decision (config only by default; settings.json, projects/, sessions/, memories/ per account unless --share) and to the names-only rule in LAYERING-SPEC §9.1; the TASK-22 part becomes a static notice.

2026-11-22 (@architect, final pass): criterion 1 ignores the blocked folders (owner decision 7 in TASK-50.1).
<!-- SECTION:NOTES:END -->
