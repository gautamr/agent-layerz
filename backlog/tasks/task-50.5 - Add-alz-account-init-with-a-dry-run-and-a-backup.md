---
id: TASK-50.5
title: Add alz account init with a dry run and a backup
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-22T01:36:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.2
parent_task_id: TASK-50
priority: low
ordinal: 21390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Creating ~/.claude-<name> and ~/.codex-<name> and linking the shared items moves the user's files aside, which makes it the riskiest part. It follows the dry-run-and-backup rule from TASK-48. By default it shares configuration only (owner decision in TASK-50.1). It can ship after the rest. Design: LAYERING-SPEC §7.13 'Shared and per-account items' and 'alz account init safety', §9.1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz account init <name> --dry-run lists every folder it would create, every item it would link and every existing item it would move aside (a rename in the same folder), and changes nothing
- [ ] #2 Without --dry-run it backs up every item it replaces only by renaming it in the same folder to <item>.alz-backup-<UTC timestamp>, never by copying and never over an existing backup, prints each backup's path, and never opens a file inside an account folder; no backup name matches .claude-* or .codex-*
- [ ] #3 Rerunning it changes nothing (idempotence test)
- [ ] #4 By default it links only claude agents, commands, skills, plugins and CLAUDE.md and codex AGENTS.md and skills, and only those that exist in the default folder; settings.json, claude projects/, codex sessions/ and memories/ are linked only with --share <item>, which prints a warning; --share accepts only settings.json, projects, sessions and memories and rejects anything else before planning
- [ ] #5 It refuses the names default and alz-blocked and the configured default_name, an account folder or shared-item parent that is a symbolic link (checked with lstat), and an account folder whose real path is the default folder; it plans everything first and changes nothing when the plan has any problem; it creates folders with mode 0700 (tests with a fake HOME)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (@architect): revised to the owner's shared-items decision and the review's init safety findings (LAYERING-SPEC §7.13, §9.1, §13 test 23).

2026-11-22 (@architect, second review): criterion 4 limits --share to settings.json, projects, sessions and memories (LAYERING-SPEC §7.13, §13 test 23).

2026-11-22 (@architect, final pass): criterion 5 also refuses alz-blocked, the name of the blocked folders that must never exist (owner decision 7 in TASK-50.1; LAYERING-SPEC §7.13).
<!-- SECTION:NOTES:END -->
