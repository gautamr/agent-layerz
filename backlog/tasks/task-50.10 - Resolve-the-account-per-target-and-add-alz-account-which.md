---
id: TASK-50.10
title: Resolve the account per target and add alz account which
status: To Do
assignee: []
created_date: '2026-11-22T03:33:00Z'
updated_date: '2026-11-22T07:26:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.9
references:
  - docs/LAYERING-SPEC.md
  - docs/target-facts.md
parent_task_id: TASK-50
priority: high
type: feature
ordinal: 27390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Third slice of TASK-50.2. The resolver and the read-only alz account which: the resolution order (an alz use choice arrives as an argument, LAYERING-SPEC §7.13 'alz use stays in the shell'), the user config and the allow state read with their safety checks, path matching and the root walk, the environment source (ALZ_SET_ markers), fail closed and what the hook would apply. Step 4 uses the previous slice's cache reader. Tests write allowed.yaml themselves; alz account allow comes in the next slice.

This is the resolver that alz run (TASK-50.3), alz use and the hook (TASK-50.6) and alz doctor (TASK-50.4) will call, so its result per target says what applies, why each step was skipped, and, when a named account fails closed, what the hook sets and which fix to print.

Out of scope: writing allowed.yaml, the allow hints in build and check, alz run, the hook.

Spec: LAYERING-SPEC §7.13 'User config', 'Allow state', 'Both files', 'Resolution', 'Paths', 'No fetch on resolve', 'A named account that cannot apply fails closed', 'Values set outside alz', §9.1, §10 'alz account which', §13 tests 19, 20 and 24. Facts: docs/target-facts.md §12 items 10, 11, 18 and 20.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz account which prints the resolved account per target, its source (use, environment, user config, layer or default; use comes from the alz shell function, environment from comparing the variable with its ALZ_SET_ marker), the reason for each skipped step and the paths of accounts.yaml and allowed.yaml; when a named account fails closed it prints 'layer step skipped (<reason>); fails closed', what the hook applies (the missing folder, the recorded account or the blocked folder) and the fix command; tests cover each source, each fail-closed case and a missing default folder (never fails)
- [ ] #2 The user config (accounts.yaml, hand-written, never written by alz) and the allow state (allowed.yaml) live outside every repository; alz ignores either with a warning when its real path is inside the current repository, the user does not own it, group or others can write it, its directory or a parent directory below $HOME, or the XDG_ variable that locates it points somewhere other than its default (~/.config or ~/.local/state, compared as real paths) while __MISE_DIFF or DIRENV_DIFF exists, the warning then naming the variable and saying that mise or direnv is active here; alz checks only that those two variables exist and never reads, decodes, prints or logs their values (decision-17; test: a marker string put in them appears in no output); an allowed.yaml entry with a relative root or an invalid account name is ignored with a warning; and alz build never writes an account value into a generated file
- [ ] #3 Resolution (shell hook, alz run, alz account which) never fetches, clones or runs git and writes nothing: for a root not in allowed.yaml it reads no file of the stack, and otherwise it reads only layers it can locate without git (see the cache index criterion), with build's validation, and checks each against the lock's content hash (TASK-51), a mismatch counting as a cache miss; a cache miss skips the layer step, and in an allowed repository that fails closed as in TASK-50.3 and TASK-50.6 (tests trace git calls and file reads)
- [ ] #4 Paths resolve with the file system's spelling: a folders entry matches whole path segments and the longest wins, an allowed entry matches only the exact real root, the root is the nearest layer.yaml with a stack key at or above the working directory (one without it is skipped), each layer.yaml read is parsed up to 1 MiB with a YAML alias limit, and an entry that does not resolve matches nothing (tests: symlink, case variant, nested manifest, a layer manifest on the way)
- [ ] #5 An account named by a layer takes effect only while allowed.yaml holds an entry for the repository's real root whose whole per-target map and declaring layer's source (a remote URL without its ref, or a local layer's real path) equal the resolved ones; any difference skips the layer step for every target, while a ref bump does not; alz account which prints the alz account allow command only for a changed entry and otherwise reports only that the layer step was skipped; accounts set in the user config need no approval (tests with allowed.yaml written by the test: no entry gives the default account, a matching entry the named one, a changed map or source skips it, a ref bump keeps it)
- [ ] #6 A declaration by a layer that may not declare is ignored with one warning by alz account which, in the resolver that alz run and the hook will use (LAYERING-SPEC §7.13 'Who may declare it', §13 test 24, in test/acceptance/24-untrusted-repo.test.ts)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 owner: when a layer.yaml on the root walk cannot be read (permission, over 1 MiB, invalid YAML), the walk stops there, treats that folder as the root, warns, and fails closed if that root is allowed; otherwise the default account applies. It never skips past an unreadable manifest.

2026-11-22 coordinator: include the rule that a non-default XDG_CACHE_HOME while mise or direnv is active is a cache miss (fails closed in an allowed repository), consistent with the owner's XDG decision.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-22T03:35:00Z

---

Owner question (affects criterion #4, the root walk; blocks the plan of this task only). The spec does not say what the walk does when a layer.yaml on the way cannot be read: over 1 MiB, past the alias limit, invalid YAML, or not a regular file. Options: (a) stop there and treat that directory as a root whose stack cannot be read: step 4 is skipped, and if that root is in allowed.yaml it fails closed like a cache miss (recommended); (b) skip it and walk on. Then a nested checkout with a broken manifest inside an allowed repository would get the outer repository's account.

---

author: @architect
created: 2026-11-22T07:25:00Z

---

decision-18 (owner, from TASK-50.9): layers inside the repository are read only to warn and never checked against the lock. Criterion #3 says 'checks each against the lock's content hash'; that now means each layer outside the repository (owner to reword if wanted). Step 4 calls readAccountDeclaration from TASK-50.9 (plan section 4): pass the real root, the parsed stack and cacheDir; map its miss codes to the 'layer step skipped (<reason>)' text; use recordedSource() for the 'recorded source still in the stack' check on a miss.

---
<!-- COMMENTS:END -->
