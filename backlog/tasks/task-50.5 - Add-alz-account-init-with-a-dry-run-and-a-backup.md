---
id: TASK-50.5
title: Add alz account init with a dry run and a backup
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-22T03:10:00Z'
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
Creates ~/.claude-<name> and ~/.codex-<name> with mode 0700. By default it shares nothing and makes no link (decision-17, amending decision-16's config-only default): every account keeps its own user-level configuration, and personal agents and skills reach alz repositories through alz build at project scope. Each shared item is opt-in with --share <item> and a warning. Linking moves the user's existing item aside, which makes this the riskiest part; it follows the dry-run-and-backup rule from TASK-48. Claude skills/ and plugins/ are linked entry by entry so that claude.ai's synced/ and .trash/ stay per account (target-facts §12.1 C4); limits of opting in are C5, C10 and O7. It can ship after the rest. Design: LAYERING-SPEC §7.13 'Shared and per-account items' and 'alz account init safety', §9.1, §10, §13 test 23; facts: docs/target-facts.md §12 item 14.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz account init <name> --dry-run lists every folder it would create, every item it would link and every existing item it would move aside (a rename in the same folder), and changes nothing
- [ ] #2 Without --dry-run it backs up every whole item it replaces only by renaming it in the same folder to <item>.alz-backup-<UTC timestamp>, never by copying and never over an existing backup, prints each backup's path, and never opens a file inside an account folder; no backup name matches .claude-* or .codex-*; an entry inside Claude's skills/ or plugins/ is never moved aside
- [ ] #3 Rerunning it with the same arguments changes nothing, except to link entries added since to a shared Claude skills/ or plugins/ (idempotence test)
- [ ] #4 Without --share it creates only the account folders, with mode 0700, and makes no link (decision-17); --share <item> accepts only agents, commands, CLAUDE.md, skills, plugins, settings.json, projects, AGENTS.md, sessions and memories (skills applies to both targets) and rejects anything else before planning; it links only items that exist in the default folder, and each shared item prints a warning naming its limits from LAYERING-SPEC §7.13 (plugins: install from the default account, C10; Codex skills: skills/.system is rewritten through the link, C5; settings.json: shares the keys that carry or pin a login; Codex sessions and memories: not yet observed, O7)
- [ ] #5 It refuses the names default and alz-blocked and the configured default_name, an account folder or shared-item parent that is a symbolic link (checked with lstat), and an account folder whose real path is the default folder; it plans everything first and changes nothing when the plan has any problem; it creates folders with mode 0700 (tests with a fake HOME)
- [ ] #6 --share skills and --share plugins for Claude create a real skills/ or plugins/ folder (mode 0700) in the account and link each entry of the default copy except synced and .trash, which stay per account (target-facts §12.1 C4); an entry that already exists in the account is left alone and listed (tests with a fake HOME)
- [ ] #7 Before --share plugins and --share projects are built, docs/target-facts.md §12 item 14 records by smoke test, with a throwaway CLAUDE_CONFIG_DIR and fake HOME, whether Claude keeps per-entry links in plugins/ after a plugin install and a linked projects/ after a session; if Claude replaces a link, that item is not built and the owner is asked
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (@architect): revised to the owner's shared-items decision and the review's init safety findings (LAYERING-SPEC §7.13, §9.1, §13 test 23).

2026-11-22 (@architect, second review): criterion 4 limits --share to settings.json, projects, sessions and memories (LAYERING-SPEC §7.13, §13 test 23).

2026-11-22 (@architect, final pass): criterion 5 also refuses alz-blocked, the name of the blocked folders that must never exist (owner decision 7 in TASK-50.1; LAYERING-SPEC §7.13).

2026-11-22 (@architect, from TASK-50.7 facts and decision-17): description and criteria 2, 3 and 4 revised to 'nothing shared by default'; criterion 6 added for per-entry links in Claude skills/ and plugins/ (C4); criterion 7 added because per-entry links in plugins/ and a linked projects/ were not tested in TASK-50.7 (target-facts §12 item 14). Backups: whole items only; entries inside skills/ or plugins/ are left alone, since a backup inside skills/ would load as a skill. --share skills applies to both targets (Codex: whole-folder link, C5).

2026-11-22 coordinator: recording per-entry plugins/ links and a linked projects/ in target-facts is criterion 7 of this task (accepted). --share takes the item name and applies to every target that has it; a per-target form is not planned.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-22T02:56:00Z

---

decision-17 (accepted 2026-11-22, amends decision-16): alz account init shares nothing by default and every shared item is opt-in with --share; warnings about values set outside alz say only that mise or direnv is active, from the presence of __MISE_DIFF or DIRENV_DIFF, never decoding them. Owner decisions recorded in TASK-50.7.

---
<!-- COMMENTS:END -->
