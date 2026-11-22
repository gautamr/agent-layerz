---
id: TASK-50.2
title: Resolve each repository's account from its layers and the user config
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-22T01:58:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.1
  - TASK-51
parent_task_id: TASK-50
priority: high
ordinal: 18390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The client layer names the expected account; a machine-local user config can override it per folder; nothing is committed. Same pattern as commit_identity (decision-13).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A layer can declare an account name, and alz lint rejects anything in that field other than a name of at most 64 characters (no paths, no secrets)
- [ ] #2 alz account which prints the resolved account per target, its source (use, environment, user config, layer or default; use comes from the alz shell function, environment from comparing the variable with its ALZ_SET_ marker), the reason for each skipped step and the paths of accounts.yaml and allowed.yaml; when a named account fails closed it prints 'layer step skipped (<reason>); fails closed', what the hook applies (the missing folder, the recorded account or the blocked folder) and the fix command; tests cover each source, each fail-closed case and a missing default folder (never fails)
- [ ] #3 The user config (accounts.yaml, hand-written, never written by alz) and the allow state (allowed.yaml) live outside every repository; alz ignores either with a warning when its real path is inside the current repository, the user does not own it, group or others can write it, its directory or a parent directory below $HOME, or mise or direnv set the XDG_ variable that locates it; an allowed.yaml entry with a relative root or an invalid account name is ignored with a warning; and alz build never writes an account value into a generated file
- [ ] #4 An account named by a layer takes effect only after alz account allow in that repository, which records the repository's real root, the whole per-target map and the declaring layer's source (a remote URL without its ref, or a local layer's real path) in allowed.yaml; until then, or after the map or the source changes (any difference skips the layer step for every target), it does not apply, while a ref bump needs no new allow; alz build and alz check print the allow command only on a terminal with no agent-session marker set, alz account which prints it only for a changed entry and otherwise reports only that the layer step was skipped, and the hook and alz run never print it; accounts set in the user config need no approval (tests: a fresh clone starts on the default account; after allow it uses the named one; a changed map or source skips it again; a ref bump keeps it)
- [ ] #5 Resolution (shell hook, alz run, alz account which) never fetches, clones or runs git and writes nothing: for a root not in allowed.yaml it reads no file of the stack, and otherwise it reads only layers it can locate without git (see the cache index criterion), with build's validation, and checks each against the lock's content hash (TASK-51), a mismatch counting as a cache miss; a cache miss skips the layer step, and in an allowed repository that fails closed as in TASK-50.3 and TASK-50.6 (tests trace git calls and file reads)
- [ ] #6 alz account allow shows the account per target, the declaring layer, its source and its revision, and asks the person to type the account name to confirm; it refuses and changes nothing when stdin is not a TTY or inside an agent session, and it has no --yes; it holds an exclusive lock file next to allowed.yaml while reading and writing, keeps entries sorted by root and replaces the entry for the same root; alz account allow --remove removes this root's entry under the same lock file, with the same terminal and agent-session checks; on a cache miss allow changes nothing and says only to run alz build (a test for each refusal, for --remove and for two concurrent runs)
- [ ] #7 Paths resolve with the file system's spelling: a folders entry matches whole path segments and the longest wins, an allowed entry matches only the exact real root, the root is the nearest layer.yaml with a stack key at or above the working directory (one without it is skipped), each layer.yaml read is parsed up to 1 MiB with a YAML alias limit, and an entry that does not resolve matches nothing (tests: symlink, case variant, nested manifest, a layer manifest on the way)
- [ ] #8 A layer that sets CLAUDE_CONFIG_DIR, CODEX_HOME, HOME, XDG_CONFIG_HOME, XDG_STATE_HOME or XDG_CACHE_HOME through settings.yaml, an MCP environment or a passthrough file fails the build naming the layer, and a declaration by a layer that may not declare is ignored with one warning by alz run, the hook and alz account which (LAYERING-SPEC §13 test 24, in test/acceptance/24-untrusted-repo.test.ts)
- [ ] #9 The repository invariant 'outside the repo, only the cache is written' in .layer/instructions/50-repo.md names allowed.yaml as the one other file alz writes, the threat-model skill's trust table gains rows for accounts.yaml and allowed.yaml, and mise run alz:build then mise run alz:check pass
- [ ] #10 A pinned path: layer is found through a cache index entry that alz build writes next to each tree@<tree> entry, keyed by the layer directory's real path and the commit SHA (for example pathref@<sha256 of the real path>-<sha>) and naming its tree; build and resolution compute the real path with the same function, resolution computes it from the manifest without git, uses the lock sha only when it is 40 or 64 hex characters, reads only the index entry for that path, and uses it only when lstat shows a regular file whose whole content is 40 or 64 hex characters; a path: layer without a ref is read in place (only its layer.yaml is used, with the LAYERING-SPEC §7.12 inside/outside test); anything else is a cache miss, and layer.lock does not change (tests: index hit, a malformed sha, another path's index entry, a malformed index entry, a symlinked index entry)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-21: the owner approved the allow step (criterion #4). Reason: repository content is untrusted, and a manifest can name an external layer that names one of the owner's accounts (LAYERING-SPEC §7.12 'Trust'). Design in TASK-50.1's notes.

2026-11-22 (@architect): criteria revised to the owner decisions and review findings recorded in TASK-50.1 (allow scope and human-only allow, fail-closed missing folder, marker sources, no fetch on resolve, path matching, user config safety, layers may not set the variables, the invariant update now belongs here because allow writes outside the repo). Spec: LAYERING-SPEC §7.13, §7.9, §9.1, §13 tests 19, 20 and 24. Settled 2026-11-22 (coordinator, second review; replaces the earlier open point): a pinned path: layer is found through a cache index that alz build writes next to each tree@<tree> entry, keyed by the layer directory's real path and the commit SHA (for example pathref@<sha256 of the real path>-<sha>) and naming its tree. Resolution computes the real path from the manifest without git, accepts the lock sha only when it is 40 or 64 hex characters, and reads only that path's index entry; an unpinned path: layer is read in place (layer.yaml only, §7.12 inside/outside test); anything else is a cache miss. layer.lock does not change, so no generated output changes. Resolution relies on cache integrity, which TASK-51 fixes. See criterion #10 and LAYERING-SPEC §7.13 'No fetch on resolve'.

2026-11-22 (@architect, final pass): criteria 2, 5, 6 and 10 revised for the fail-closed output of alz account which, the content-hash check on resolution (TASK-51), alz account allow --remove, the alz build-only hint and the index entry checks (LAYERING-SPEC §7.13, §10, §13 tests 19 and 20). Dependencies left as set (TASK-50.1, TASK-51).

2026-11-22 (@implementer, from TASK-51 review): build treats a lock with no entry for a source, or an entry whose sha differs, as 'skip the content check' because new content is expected. Account resolution must not: anything it cannot verify (no lock entry, a different sha, a missing or malformed field, an unreadable entry, a layer.yaml that is not a regular file) is a cache miss that fails closed. Use lockedEntryFor and verifyCachedLayer (src/resolve/cache-integrity.ts), hash with hashLayerDir (src/resolve/layer-hash.ts) before parsing layer.yaml, and for tree@ entries it cannot run git, so it relies on the lock hash only.
<!-- SECTION:NOTES:END -->
