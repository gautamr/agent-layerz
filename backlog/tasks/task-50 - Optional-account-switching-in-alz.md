---
id: TASK-50
title: Optional account switching in alz
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-22T02:56:00Z'
labels:
  - area:accounts
milestone: m-3
dependencies: []
priority: high
ordinal: 16390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-15 (replaces decision-2), amended by decision-16 and decision-17: developers adopt one tool more readily than two, and alz already knows which client a repository belongs to. alz chooses the account folder per repository; it never reads, copies or prints logins. Design: LAYERING-SPEC §7.13.
Boundaries:
- Off by default: with nothing configured, nothing changes.
- alz sets only each target's account variable (Claude CLAUDE_CONFIG_DIR, Codex CODEX_HOME), defined by the target's adapter.
- A client layer may name the account its repositories expect, like commit_identity (decision-13); it applies only after a person runs alz account allow in that repository (alz account allow --remove withdraws it). The owner's override is machine-local user config, never committed.
- A named account that cannot apply fails closed (decision-16, owner decisions 7 and 8 in TASK-50.1): alz run and alz use fail before starting; the shell hook warns and always sets the variable, to the missing named folder, to the recorded account on a cache miss while the recorded source is still in the stack, or to a blocked folder that alz never creates (~/.claude-alz-blocked, ~/.codex-alz-blocked; Claude creates a missing folder and starts logged out, Codex refuses to start, LAYERING-SPEC §12 item 18) when the declaration changed or the recorded source left the stack. A tool never starts on the default account or on a newly declared one instead.
- A value set outside alz (one that differs from alz's marker) is left alone with a 'set outside alz' warning instead of being overridden; the warning adds 'mise or direnv is active here' when __MISE_DIFF or DIRENV_DIFF exists, and alz never reads their values (decision-17).
- alz account init shares nothing by default; every shared item is opt-in with --share (decision-17).
- Known limits, recorded in LAYERING-SPEC §7.13 from docs/target-facts.md §12: Claude logins kept outside the account folder are shared by every folder (C3), and Codex MCP OAuth logins are shared across CODEX_HOME folders by default (C2, decision deferred by the owner).
Out of scope: credentials and logins; the owner's dotfiles (claude-as, client-init), which move over separately once this ships.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every subtask is Done
- [ ] #2 After alz account allow in each repository, and with the shell hook installed, plain 'claude' and 'codex' in two repositories of two different clients start on the two accounts their layers name, and in a repository with no account configured they start on the default account (recorded end-to-end run)
- [ ] #3 A test proves alz never opens a file inside an account folder: it lists names only
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (@architect): description aligned with decision-17 and the TASK-50.7 facts (sharing opt-in, presence-only mise/direnv wording, known limits C2 and C3).
<!-- SECTION:NOTES:END -->
