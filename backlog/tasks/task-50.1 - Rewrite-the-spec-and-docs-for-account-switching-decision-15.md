---
id: TASK-50.1
title: Rewrite the spec and docs for account switching (decision-15)
status: Done
assignee:
  - '@claude'
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-22T01:38:00Z'
labels:
  - area:accounts
  - area:docs
milestone: m-3
dependencies: []
parent_task_id: TASK-50
priority: high
ordinal: 17390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
LAYERING-SPEC §2 lists 'managing credentials or accounts' as a non-goal, and doc-1, FRAMEWORK-GAPS and the README name a separate workspace-accounts project. Settle the design in the spec before code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 LAYERING-SPEC states the goal (choose the account folder) and keeps the non-goal (credentials and logins), with the layer field, the user config file and its path, the resolution order and what happens when the chosen account folder is missing (fail closed)
- [x] #2 doc-1, FRAMEWORK-GAPS.md and README.md no longer describe a separate account project, and each points to decision-15
- [x] #3 The spec lists, per target adapter, the variable it sets and what is shared between account folders
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. LAYERING-SPEC: status line; §2 new goal 8 (choose the account folder) and the non-goal narrowed to credentials and logins; new §7.13 Account selection (layer field, who may declare it, the same rule as §7.12 because repository content is untrusted, user config path and shape, resolution order, fallback, conflicts with mise or direnv, per-target variable table, shared and per-account items); §8.4 and §9.1 point to §7.13; §10 planned rows for alz run, alz use, alz activate, alz account which and alz account init; §12 a fact to verify for each variable.
2. doc-1: phase 3 line and the Decisions bullet (backlog doc update).
3. FRAMEWORK-GAPS.md lines 50, 319 and 332. README: one planned-feature pointer to decision-15.
Checks: grep shows no 'workspace-accounts' outside superseded decision-2 and the board history; every AC1 element is present in §7.13; mise run lint and the tests still pass (docs only); confidentiality grep.
Risk: the spec describes unbuilt behaviour, so every new part is marked planned with its task ID.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-21: started from a client hub session; the hub's guard hook blocks file edits outside the hub, so no file was changed. Released for a session started in this repository. Drafted design for LAYERING-SPEC (write it there):
- New §7.13 Account selection (planned: TASK-50, decision-15). Off by default. Account name ^[a-z0-9][a-z0-9-]*$; 'default' always means the default account.
- Per target: claude CLAUDE_CONFIG_DIR (default: unset, ~/.claude plus ~/.claude.json; named ~/.claude-<name>); codex CODEX_HOME (default: unset, ~/.codex; named ~/.codex-<name>).
- A layer may declare account: <name> or a per-target map, under the same who-may-declare rule as commit_identity (§7.12: first stack entry, or one private kind: client layer outside the repository; client wins).
- User config ${XDG_CONFIG_HOME:-~/.config}/agent-layerz/accounts.yaml, machine-local, never committed: default_name (what the default account is called here), folders (path to account, or a per-target map), allowed (repositories whose layer-named account was accepted).
- Trust: repository content is untrusted and a manifest can name an external layer that names your account (the §7.12 'Trust' weakness), so a layer-named account applies only after 'alz account allow' in that repository (direnv-style). Proposed addition to TASK-50.2; needs the owner's yes.
- Resolution, first match wins: 1 alz run --account; 2 alz use --account (terminal, until --reset); 3 longest user-config folders path; 4 layer declaration if allowed; 5 default. 'alz account which' shows the result and its source.
- Missing folder: an explicit choice (1, 2) fails and lists account folders by name; an implicit one (3, 4) falls back to the default with a warning.
- Conflicts: a variable already set by mise, direnv or the user is left alone, with one warning naming both sources; an explicit --account wins.
- Switching: eval "$(alz activate zsh|bash)" hook plus the alz shell function alz use needs; alz run needs no hook.
- Shared items linked by alz account init: claude agents, commands, skills, plugins, settings.json, CLAUDE.md, projects; codex AGENTS.md, skills, memories, sessions. Per account: logins, claude .claude.json (trust, per-project state, MCP logins), prompt history, file checkpoints; codex config.toml.
- Also: §2 add goal 8 and narrow the non-goal to credentials and logins; §8.4 points to §7.13; §9.1 allows two planned writes outside the repo on explicit command (account allow writes the user config; account init with --dry-run and backup); §10 planned rows for run, use, activate, account which, account allow, account init; §12 item 10 (the variables are read only at start; where login, trust and state live; which items work as links); status line.
- The repo instruction invariant 'outside the repo, only the cache is written' changes with TASK-50.2 (alz account allow already writes outside the repo there), not now.

2026-11-21: owner approved the trust gate: a layer-named account applies only after 'alz account allow' in that repository. Spec includes it; TASK-50.2 gets the criterion.

2026-11-21: wrote the drafted design, with the trust gate as settled (owner approved 'alz account allow').
- LAYERING-SPEC: status line; §2 goal 8 and the non-goal narrowed to credentials and logins (plus a note that alz run only starts one tool); §3.1 kind: client now also covers the account; new §7.13 Account selection (layer field, names, per-target variable table, who may declare via §7.12, user config path and shape, trust gate, resolution order with step 4 'layer declaration, if allowed', missing-folder fallback, mise/direnv conflicts, switching, shared vs per-account table, no generated output, layer-format version note); §8.4 and §9.1 (planned writes on explicit command: account allow, account init) point to §7.13; §10 planned rows for run, use, activate, account which, account allow, account init, plus a planned TASK-50.4 sentence on doctor; §12 items 10 (CLAUDE_CONFIG_DIR) and 11 (CODEX_HOME). Every unbuilt part names its task.
- doc-1 phase 3 line and Decisions bullet rewritten for decision-15 (backlog doc update). FRAMEWORK-GAPS.md lines 50, 319, 332 point to decision-15 (private file, git-excluded). README Status: one planned-feature pointer to decision-15 and §7.13.
- Choices where the draft was open: client layer's account replaces the first layer's whole value (per-target map included), as in §7.12; a target missing from a map skips that step; shared items link to the default account folder's copy; default_name means a layer or --account naming it gets the default account; folders match the working directory or a parent, longest path wins; allowed holds the repository root, compared as real paths, and approval is per repository, not per value (stated as a limit). §5.1 is unchanged: it lists the field when TASK-50.2 ships it.
Checks: grep -rn workspace-accounts (excl. node_modules, .git) -> only decision-2, TASK-22 and TASK-50.1 history; mise run lint -> exit 0 (biome 109 files, tsc clean); mise run test -> 26 files, 135 passed, 1 skipped, exit 0; mise run alz:check -> 58 generated files match, exit 0; confidentiality grep of tracked additions -> no hits. Not committed.

2026-11-21 owner decisions after review: (1) allow records the repo real path, the account per target and the declaring layer's source; any change skips step 4, warns old/new, and asks for allow again. (2) A missing account folder fails closed: alz run fails before start, the hook leaves the variable unset and warns (amends decision-15's fallback). (3) account init shares config only by default (claude agents, commands, skills, plugins, CLAUDE.md; codex AGENTS.md, skills); settings.json, claude projects/, codex sessions/ and memories/ are opt-in with --share <item> and a warning. (4) allow is human only: shows account, layer, source, revision; confirms on a TTY; refuses without a TTY or inside an agent session; no --yes.

2026-11-22 (@architect): revised after review to the owner decisions above and the review findings.
- LAYERING-SPEC: §7.13 rewritten (allow records root, account per target and source and is human only; missing folder fails closed; config-only sharing by default with --share; no fetch on resolve; marker variables ALZ_SET_<VARIABLE> and ALZ_USE_<TARGET>; evaluated hook output rules; allowed.yaml in XDG_STATE_HOME, accounts.yaml hand-written; file safety; path matching; who-may-declare edge cases and hub sessions; account init safety; trust is per account; Codex needs only CODEX_HOME; names at most 64 characters). §7.9 planned rule: layers may not set CLAUDE_CONFIG_DIR, CODEX_HOME, HOME, XDG_CONFIG_HOME or XDG_STATE_HOME. §8.4 shared configuration only. §9.1 names-only operations and planned writes. §9.2 public account note. §10 rows revised. §12 items 10 to 20. §13 planned tests 19 to 24 (24 extends test/acceptance/24-untrusted-repo.test.ts). README links §7.13 (decision-15 named).
- Board: TASK-50 description and AC 2; TASK-50.2 AC 1-4 reworded, AC 5-9 added; TASK-50.3 AC 3 reworded, AC 5 added; TASK-50.4 description, AC 1-2 reworded, AC 4 added; TASK-50.5 description, AC 1-2 reworded, AC 4-5 added; TASK-50.6 AC 1 and 4 reworded, AC 5-6 added; TASK-44 AC 3 names TASK-50.6; this task's AC 1 says fail closed instead of fallback, re-checked against the spec; the invariant note now says TASK-50.2.
- Choices: XDG_STATE_HOME added to the variables layers may not set (it locates allowed.yaml); allow reads the stack like resolution (cache only) and asks for alz build on a miss; the source is recorded as the manifest writes it, so a github:/git: ref bump needs a new allow; for a repository not allowed, alz account which reports step 4 as skipped without reading the stack, and build and check give the specific hint.
Checks: workspace-accounts grep -> only decision-2, TASK-22 and TASK-50.1 history; mise run lint -> exit 0; mise run test -> 26 files, 135 passed, 1 skipped, exit 0; mise run alz:check -> 58 files match, exit 0; confidentiality grep -> no hits. Not committed.

2026-11-22 owner decisions after the second review: (5) the hook, when a named account cannot apply (folder missing, or in an allowed repo a cache miss or changed declaration), sets the variable to the named folder's path so the tool starts logged out, never on the default account; alz run and alz use fail before starting; depends on the §12 fact on missing folders, and if a tool rejects a missing folder the hook warns and leaves it unset. (6) D1-D5 are recorded as decision-16, which amends decision-15. Mechanism chosen by the coordinator: a pinned path: layer is found through a build-written cache index keyed by the layer's real path and commit SHA (no lock change).

2026-11-22 (@architect, second review round): applied S1 to S3 and the text fixes.
- LAYERING-SPEC: status date 2026-11-22. §7.13 rewritten again: decision-16 cited (header, allow state, allow, sharing, fail closed); hook fails closed by setting the variable to the named account's folder (S1, with the §12 item 18 caveat), and alz run and alz use fail and name the fix; cache index for pinned path: layers, unpinned path: layers read in place, lock sha only 40 or 64 hex, lock unchanged, TASK-51 relied on (S3); allow confirms by typing the account name, honest limit on agents that write outside the repo, personal-layer deny rules and a sandbox as the real protection; source recorded as remote URL without ref or local real path, whole per-target map compared, ref bump needs no new allow; alz use choice held in unexported shell variables, exported ALZ_USE_ never read, names from the environment or allowed.yaml validated; XDG file safety (parent directories, mise or direnv set XDG_), which prints both paths, allowed.yaml entries validated, allow lock file and sorted entries; root walk skips layer.yaml without stack, 1 MiB and alias limit; missing default folder never fails; build and check hint only on a TTY without agent marker, which only on a changed entry; --share accepts four items. §7.9 adds XDG_CACHE_HOME and the TASK-51 note. §9.1 'only read operations', points to planned writes, cache index is no exception. §10 rows revised. §12 note on cache integrity (TASK-51) and item 21. §13 tests 19 to 24 revised (forged ALZ_USE_CLAUDE, missing default folder, --share rejection, line break fixed).
- Board: TASK-50 description (decision-16, hook fail closed); TASK-50.2 all criteria revised, #10 added (cache index), open-point note replaced by the settled mechanism; TASK-50.3 AC 3 and 5; TASK-50.5 AC 4; TASK-50.6 AC 1, 2, 4, 5 and 6.
- Interpretation to confirm: when step 4 is skipped in an allowed repository (cache miss or changed declaration), the hook sets the variable to the folder of the account recorded in allowed.yaml, never a newly declared one that was not allowed; the tool then starts on that allowed account, not logged out, unless its folder is missing.
Checks: workspace-accounts grep -> only decision-2, TASK-22 and TASK-50.1 history; mise run lint -> exit 0; mise run test -> 26 files, 135 passed, 1 skipped, exit 0; mise run alz:check -> 58 files match, exit 0; confidentiality grep -> 0 hits; git diff --stat -> 11 files, 518 insertions, 47 deletions. Not committed.

2026-11-22 owner decisions after the final pass: (7) in an allowed repository whose declaration changed, or whose recorded source is no longer in the stack, the hook sets each named target's variable to a folder that never exists (~/.claude-alz-blocked, ~/.codex-alz-blocked; alz never creates it and account init refuses the name), so the tool starts logged out; only a cache miss with the recorded source still in the stack keeps the recorded account; add alz account allow --remove for one root. (8) The 'leave it unset if a tool rejects a missing folder' fallback is dropped: the hook always sets the variable, and a tool that refuses to start counts as failing closed.

2026-11-22 (@architect, final pass): applied owner decisions 7 and 8 and the final review fixes.
- LAYERING-SPEC §7.13: fail-closed bullet rewritten (the hook always sets the variable: missing named folder, recorded account on a cache miss with the recorded source still in the stack, or the blocked folder ~/.claude-alz-blocked / ~/.codex-alz-blocked on a changed declaration or a source that left the stack; never a newly declared or the default account; the unset caveat dropped); alz account which fail-closed output; alz account allow --remove under the same lock file, with the same terminal and agent-session checks; cache-miss hint says only alz build; index entry used only as a regular file (lstat) holding 40 or 64 hex characters, real path computed by one shared function; each layer read checked against the lock's content hash, mismatch = cache miss; init refuses alz-blocked. §10 rows for doctor, run, activate, which, allow and init. §12 item 18 records the behaviour instead of deciding it. §13 tests 20 to 23 (test 22 uses the owner's wording).
- Board: TASK-50 description; TASK-50.2 AC 2, 5, 6 and 10 (dependencies untouched); TASK-50.3 AC 3; TASK-50.4 AC 1; TASK-50.5 AC 5; TASK-50.6 AC 5; TASK-51 AC 4 added.
- Choice: alz account allow --remove gets the same terminal and agent-session checks as allow, because removing an entry sends the repository back to the default account.
Checks: workspace-accounts grep -> only decision-2, TASK-22 and TASK-50.1 history; mise run lint -> exit 0; mise run test -> 26 files, 135 passed, 1 skipped, exit 0; mise run alz:check -> 58 files match, exit 0; confidentiality grep -> 0 hits; git diff --stat -> 11 tracked files, 565 insertions, 47 deletions (TASK-51 and decision-16 are new, untracked files). Not committed.

2026-11-22 coordinator: allow --remove keeps the same terminal and agent-session checks as allow. Doctor warns when a blocked folder exists (spec §7.13 and TASK-50.4 AC). Implementer note for TASK-50.2: hashing whole layers on every cd may need a cheap result cache.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
LAYERING-SPEC now specifies optional account selection (decision-15, amended by decision-16): goal 8 and a narrowed non-goal in §2; new §7.13 (layer field and who may declare, user config accounts.yaml and alz-only allowed.yaml, resolution order with a human-only allow gate scoped to repo, account per target and layer source, no fetch on resolve with a cache index for pinned path layers, fail closed including a blocked folder, marker variables and eval-safe hook output, path matching, per-target variable table, config-only sharing with --share opt-in, account init safety); §7.9 forbids layers setting account and XDG variables; §8.4, §9.1, §9.2, §10, §12 items 10-21 and §13 planned tests 19-24. doc-1, README (links §7.13) and the private FRAMEWORK-GAPS.md point to decision-15. Board: TASK-50 and 50.2-50.6 criteria aligned, TASK-44 cross-reference fixed, TASK-51 filed for cache integrity (TASK-50.2 depends on it). Reviewed three rounds by reviewer and security-reviewer; owner decided D1-D4 and (5)-(8). Checks: lint, 135 tests passed (1 skipped), alz:check, workspace-accounts grep and confidentiality grep.
<!-- SECTION:FINAL_SUMMARY:END -->
