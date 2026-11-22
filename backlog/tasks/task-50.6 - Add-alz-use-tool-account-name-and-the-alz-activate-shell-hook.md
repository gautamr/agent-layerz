---
id: TASK-50.6
title: Add alz use <tool> --account <name> and the alz activate shell hook
status: To Do
assignee: []
created_date: '2026-11-21T18:04:00Z'
updated_date: '2026-11-22T01:35:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.3
parent_task_id: TASK-50
priority: high
ordinal: 22390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The second way to switch: alz use overrides the account for the current terminal, with the same shape as alz run. A Node CLI cannot change its parent shell's environment, so like mise it needs the hook: eval "$(alz activate zsh)" defines an alz shell function and switches accounts automatically on cd.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz activate zsh and alz activate bash print a hook that runs when loaded and at each prompt where the working directory changed; after a cd into a configured repository it sets the account variables and their ALZ_SET_ markers, and unsets them on leaving (shell test for both)
- [ ] #2 With the hook loaded, alz use <tool> --account <name> sets that tool's account for the current terminal until alz use <tool> --reset, and a cd does not undo it; the choice lives in unexported variables of the alz shell function, and a forged exported ALZ_USE_CLAUDE changes nothing
- [ ] #3 Without the hook, alz use changes nothing, prints the eval line to add, and exits non-zero
- [ ] #4 When the account variable holds a value that differs from its ALZ_SET_ marker, the hook leaves it and prints one warning that it was set outside alz, naming mise or direnv only where LAYERING-SPEC §12 item 20 confirms how to detect them
- [ ] #5 When a named account cannot apply, the hook warns and always sets the variable of each named target: to the missing named folder, to the recorded account on a cache miss while the recorded source is still in the stack, or to the blocked folder (~/.claude-alz-blocked, ~/.codex-alz-blocked, which alz never creates) when the declaration changed or the recorded source is no longer in the stack; it never sets a newly declared account or the default account, and a tool that then starts logged out or refuses to start has failed closed; alz use fails before changing anything and exits non-zero (shell test for each case)
- [ ] #6 The evaluated output contains only export NAME='value' and unset NAME lines, for the account variables and the ALZ_SET_ markers; each value is built from $HOME and a validated account name and single-quoted with every ' written as '\''; every message goes to stderr, sanitised as in LAYERING-SPEC §9.4; the activate script quotes every expansion (shell test in zsh and bash: a directory named $(touch pwned) and a layer name containing ESC create no file and print no control character)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (@architect): revised to the review findings on evaluated hook output, the marker variables (ALZ_SET_<VARIABLE>, ALZ_USE_<TARGET>) and the owner's fail-closed decision (LAYERING-SPEC §7.13 'Shell hook', 'Values set outside alz', §13 test 22).

2026-11-22 (@architect, second review): criterion 5 follows the owner's hook fail-closed decision (decision-16, S1 in TASK-50.1); criterion 2 keeps the alz use choice out of the environment; criteria 1, 4 and 6 name the ALZ_SET_ markers only (LAYERING-SPEC §7.13, §13 test 22).

2026-11-22 (@architect, final pass): criterion 5 follows owner decisions 7 and 8 (blocked folder; the hook always sets the variable, no unset fallback). LAYERING-SPEC §7.13, §13 test 22.
<!-- SECTION:NOTES:END -->
