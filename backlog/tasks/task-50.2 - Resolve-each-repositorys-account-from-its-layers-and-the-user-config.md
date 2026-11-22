---
id: TASK-50.2
title: Resolve each repository's account from its layers and the user config
status: To Do
assignee:
  - '@claude'
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-22T03:38:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.1
  - TASK-50.7
  - TASK-51
  - TASK-50.8
  - TASK-50.9
  - TASK-50.10
  - TASK-50.11
parent_task_id: TASK-50
priority: high
ordinal: 18390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The client layer names the expected account; a machine-local user config can override it per folder; nothing is committed. Same pattern as commit_identity (decision-13).

Split on 2026-11-22 into four slices, one pull request each, in this order: TASK-50.8 (the account layer field, who may declare it, layers that set account variables), TASK-50.9 (reading a stack's declaration from the layer cache without git, and the pathref@ cache index), TASK-50.10 (the resolver and alz account which), TASK-50.11 (alz account allow and --remove, the allow hints, the repository invariant and the threat model). This task is now the parent that points to them: it holds the owner decisions in its notes and is done when all four are. LAYERING-SPEC parts tagged '(TASK-50.2)' mean these four slices. TASK-50.3, TASK-50.4 and TASK-50.5 depend on this task, so they wait for all four.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 TASK-50.8, TASK-50.9, TASK-50.10 and TASK-50.11 are Done; together they carry this task's former criteria #1 to #10 (mapping in the notes)
- [ ] #2 LAYERING-SPEC §7.13, §9.1 and §10 no longer mark the parts tagged TASK-50.2 as planned, and LAYERING-SPEC §13 tests 19, 20 and 24 (account cases) pass in mise run test
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Parent of four slices; no code is written under this task. Work them in order, one pull request each:
1. TASK-50.8: the account field in layer.yaml, who may declare it (build errors, init warnings), and the build error for layers that set CLAUDE_CONFIG_DIR, CODEX_HOME, HOME or an XDG_ cache/config/state variable. Planned in that task.
2. TASK-50.9: the no-git cache reader (git cache entries, pathref@ index for pinned path: layers, unpinned path: layers in place), its content-hash check, and the declaration it yields. Plan when picked up.
3. TASK-50.10: accounts.yaml and allowed.yaml reading and safety, root walk and path matching, resolution order, fail closed, alz account which. Plan when picked up.
4. TASK-50.11: alz account allow and --remove, build and check hints, 50-repo.md invariant, threat-model rows. Plan when picked up.
Done when: every slice is Done, LAYERING-SPEC no longer marks the TASK-50.2 parts as planned, and §13 tests 19, 20 and 24 (account cases) pass.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-21: the owner approved the allow step (criterion #4). Reason: repository content is untrusted, and a manifest can name an external layer that names one of the owner's accounts (LAYERING-SPEC §7.12 'Trust'). Design in TASK-50.1's notes.

2026-11-22 (@architect): criteria revised to the owner decisions and review findings recorded in TASK-50.1 (allow scope and human-only allow, fail-closed missing folder, marker sources, no fetch on resolve, path matching, user config safety, layers may not set the variables, the invariant update now belongs here because allow writes outside the repo). Spec: LAYERING-SPEC §7.13, §7.9, §9.1, §13 tests 19, 20 and 24. Settled 2026-11-22 (coordinator, second review; replaces the earlier open point): a pinned path: layer is found through a cache index that alz build writes next to each tree@<tree> entry, keyed by the layer directory's real path and the commit SHA (for example pathref@<sha256 of the real path>-<sha>) and naming its tree. Resolution computes the real path from the manifest without git, accepts the lock sha only when it is 40 or 64 hex characters, and reads only that path's index entry; an unpinned path: layer is read in place (layer.yaml only, §7.12 inside/outside test); anything else is a cache miss. layer.lock does not change, so no generated output changes. Resolution relies on cache integrity, which TASK-51 fixes. See criterion #10 and LAYERING-SPEC §7.13 'No fetch on resolve'.

2026-11-22 (@architect, final pass): criteria 2, 5, 6 and 10 revised for the fail-closed output of alz account which, the content-hash check on resolution (TASK-51), alz account allow --remove, the alz build-only hint and the index entry checks (LAYERING-SPEC §7.13, §10, §13 tests 19 and 20). Dependencies left as set (TASK-50.1, TASK-51).

2026-11-22 (@implementer, from TASK-51 review): build treats a lock with no entry for a source, or an entry whose sha differs, as 'skip the content check' because new content is expected. Account resolution must not: anything it cannot verify (no lock entry, a different sha, a missing or malformed field, an unreadable entry, a layer.yaml that is not a regular file) is a cache miss that fails closed. Use lockedEntryFor and verifyCachedLayer (src/resolve/cache-integrity.ts), hash with hashLayerDir (src/resolve/layer-hash.ts) before parsing layer.yaml, and for tree@ entries it cannot run git, so it relies on the lock hash only.

2026-11-22 (@architect, from TASK-50.7 facts and decision-17): criterion 3 replaces 'mise or direnv set the XDG_ variable' (which needed decoding their records) with a presence check: the XDG_ variable points away from its default while __MISE_DIFF or DIRENV_DIFF exists; their values are never read (open question to the owner in TASK-50.7: ignore, as written, or only warn). Criterion 6 names the six agent-session markers (target-facts §12.1 C7). Criterion 8 names Codex shell_environment_policy set/exclude (C12). Spec: LAYERING-SPEC §7.9, §7.13 'Both files', 'Agent-session markers', 'Values set outside alz', 'What a session passes on', §13 tests 19, 20 and 24. Tests must remove __MISE_DIFF, DIRENV_DIFF and the markers from the environment unless a case sets them, because mise run test passes __MISE_DIFF on (§13 intro).

2026-11-22 owner approved the XDG rule as written: when an XDG_ variable that locates accounts.yaml or allowed.yaml points away from its default while mise or direnv is active, alz ignores that file and warns.

2026-11-22 (@architect): split into TASK-50.8 to TASK-50.11. Why: one pull request would have held build-time validation, a no-git cache reader with a new cache index, a new user-config area with file-safety checks, the resolver and two new commands (estimated at more than 900 lines of source and as much test), each with its own security review. Where the former criteria went:
- #1 -> TASK-50.8 #1 (verbatim).
- #2 -> TASK-50.10 #1 (verbatim).
- #3 -> TASK-50.10 #2 (verbatim).
- #4 -> split: what resolution applies (allowed entry, changed map or source, ref bump, user config needs no approval, which prints the command only for a changed entry) -> TASK-50.10 #5, tested with an allowed.yaml the test writes; the allow command recording the entry, the build and check hints, and the end-to-end tests -> TASK-50.11 #2.
- #5 -> TASK-50.10 #3 (verbatim); the reader's own part restated as TASK-50.9 #2.
- #6 -> TASK-50.11 #1 (verbatim).
- #7 -> TASK-50.10 #4 (verbatim).
- #8 -> split: the build error -> TASK-50.8 #3 (wording unchanged); 'ignored with one warning' -> TASK-50.10 #6 (alz run and the hook get it from the same resolver in TASK-50.3 and TASK-50.6).
- #9 -> TASK-50.11 #3 (verbatim).
- #10 -> TASK-50.9 #1, verbatim except 'tree@<tree>' -> 'tree2@<tree>': TASK-51 renamed pinned cache entries, and LAYERING-SPEC §6 and §7.13 already say tree2@ (editorial).
- Added: TASK-50.8 #2 (who may declare, its build errors and the init warning: LAYERING-SPEC §7.13 'Who may declare it' and §13 test 24, which #8 cites but no criterion stated) and TASK-50.9 #3 (the reader's result, which TASK-50.10 and TASK-50.11 build on).
Fact recorded for TASK-50.8: docs/target-facts.md §12 item 15, new row 'Codex: policy patterns' (SOURCE, tag rust-v0.169.0): exclude, include_only and the newer keyed filters table are case-insensitive wildmatch patterns (* and ?); a filters entry set to exclude drops a variable like exclude does; set keys are exact on Unix and compared ignoring ASCII case on Windows.

2026-11-22 owner: hold any release with the account field until TASK-50.11 is Done (releases are CI-only and the pipeline does not exist yet anyway).
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-22T02:56:00Z

---

decision-17 (accepted 2026-11-22, amends decision-16): alz account init shares nothing by default and every shared item is opt-in with --share; warnings about values set outside alz say only that mise or direnv is active, from the presence of __MISE_DIFF or DIRENV_DIFF, never decoding them. Owner decisions recorded in TASK-50.7.

---

author: @architect
created: 2026-11-22T02:58:00Z

---

Owner question (blocks AC 3 only). decision-17 forbids decoding __MISE_DIFF/DIRENV_DIFF, so the old rule 'ignore accounts.yaml/allowed.yaml when mise or direnv set the XDG_ variable that locates it' can no longer be evaluated. Options: (a) as now written in LAYERING-SPEC §7.13 'Both files': ignore the file with a warning when its XDG_ variable points away from its default (~/.config, ~/.local/state) while either variable exists; keeps the old strictness, cost: a person with a custom XDG_CONFIG_HOME who uses mise loses the user config while mise is active. (b) Read the file and only warn; the other checks (inside the repo, owner, modes) remain. (c) Drop the condition. Recommendation: (a). Remove this label once decided.

---

author: @architect
created: 2026-11-22T03:35:00Z

---

Owner question (release timing; blocks no slice). TASK-50.8 ships the `account` field and the new build error, but a declared account has no effect until TASK-50.11. Options: (a) cut no release between TASK-50.8 and TASK-50.11, so the minor release ships the whole feature (recommended); (b) allow a release in between, with a release note saying the field is validated but not used yet. The new build error is breaking either way and needs a release note.

---
<!-- COMMENTS:END -->
