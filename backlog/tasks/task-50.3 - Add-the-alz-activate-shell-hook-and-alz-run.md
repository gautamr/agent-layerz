---
id: TASK-50.3
title: Add alz run <tool> [--account <name>]
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-22T09:52:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.2
parent_task_id: TASK-50
priority: high
ordinal: 19390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The first way to switch accounts, needing no shell hook: alz run starts one tool on the repository's resolved account, or on the account named with --account. It replaces the owner's claude-as for one-off runs, for example when an account hits its usage limit. The commands share one shape: alz run <tool> --account <name> now, alz use <tool> --account <name> in TASK-50.6.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz run <tool> [args] starts the tool with the resolved account and passes the arguments and exit code through unchanged
- [ ] #2 alz run <tool> --account <name> [args] uses that account for this run only, whatever the layers say; --account default uses the default account
- [ ] #3 alz run fails closed before starting anything, exits non-zero and never uses the default account or a newly declared one instead when the tool or account is unknown, when the account's folder does not exist whether named with --account or resolved from the user config or a layer (it lists the account folders that exist by name only), or when in an allowed repository a cache miss (it says to run alz build), a changed declaration or a recorded source no longer in the stack (it says to run alz account allow) skips the layer step
- [ ] #4 Tests cover claude and codex, with and without --account
- [ ] #5 An account variable whose value differs from its ALZ_SET_ marker (set outside alz) is passed to the tool unchanged with one 'set outside alz' warning, which adds 'mise or direnv is active here' when __MISE_DIFF or DIRENV_DIFF exists, unless --account is given; alz checks only that those two variables exist and never reads their values (decision-17; test: a marker string in them appears in no output); an alz use choice arrives only as --account from the alz shell function, and an exported ALZ_USE_<TARGET> is never read (test: a forged ALZ_USE_CLAUDE changes nothing)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (@architect): criterion 3 now fails closed for resolved accounts too (owner decision in TASK-50.1, amending decision-15's fallback); criterion 5 added for values set outside alz (LAYERING-SPEC §7.13 'Values set outside alz', §13 test 21).

2026-11-22 (@architect, second review): criterion 3 fails closed in every case where a named account cannot apply (decision-16); criterion 5 drops the exported ALZ_USE_ variable (LAYERING-SPEC §7.13 'alz use stays in the shell', §13 test 21).

2026-11-22 (@architect, final pass): criterion 3 also covers a recorded source that left the stack (owner decision 7 in TASK-50.1).

2026-11-22 (@architect, from TASK-50.7 facts and decision-17): criterion 5 gives the warning text and the presence-only mise/direnv check (target-facts §12.1 C11). Facts for the missing-folder case: Claude creates the folder (0755) and starts logged out, Codex refuses to start (LAYERING-SPEC §7.13, §12 item 18); alz run fails before either. Spec: §7.13 'Values set outside alz', §13 test 21.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-22T02:56:00Z

---

decision-17 (accepted 2026-11-22, amends decision-16): alz account init shares nothing by default and every shared item is opt-in with --share; warnings about values set outside alz say only that mise or direnv is active, from the presence of __MISE_DIFF or DIRENV_DIFF, never decoding them. Owner decisions recorded in TASK-50.7.

---

author: @architect
created: 2026-11-22T03:35:00Z

---

TASK-50.2 was split on 2026-11-22 into TASK-50.8 to TASK-50.11. This task still depends on TASK-50.2, which is Done only when all four slices are. The resolver it calls is TASK-50.10. Its result also lists declarations ignored because their layer may not declare (LAYERING-SPEC §13 test 24: "alz run, the hook and alz account which ignore it with one warning"), so this task's tests should include one such case.

---

author: @architect
created: 2026-11-22T09:52:00Z

---

From planning TASK-50.11 (owner question 5 there): LAYERING-SPEC §7.13 says 'The shell hook and alz run never print [the alz account allow command]', while §7.13 'fails closed' and §10 say alz run names alz account allow as the fix for a changed declaration or source. Proposed reading, applied to the spec in TASK-50.11's PR: alz run never prints the allow hint for a repository that is not allowed yet, and names alz account allow only as its fail-closed fix. Check TASK-50.11's comment for the owner's answer before planning this task.

---
<!-- COMMENTS:END -->
