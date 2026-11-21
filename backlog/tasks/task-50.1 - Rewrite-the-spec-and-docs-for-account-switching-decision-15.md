---
id: TASK-50.1
title: Rewrite the spec and docs for account switching (decision-15)
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-21T18:08:00Z'
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
- [ ] #1 LAYERING-SPEC states the goal (choose the account folder) and keeps the non-goal (credentials and logins), with the layer field, the user config file and its path, the resolution order and the fallback
- [ ] #2 doc-1, FRAMEWORK-GAPS.md and README.md no longer describe a separate account project, and each points to decision-15
- [ ] #3 The spec lists, per target adapter, the variable it sets and what is shared between account folders
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
- The repo instruction invariant 'outside the repo, only the cache is written' changes with TASK-50.5, not now.
<!-- SECTION:NOTES:END -->
