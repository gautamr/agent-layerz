---
id: TASK-50
title: Optional account switching in alz
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-22T01:36:00Z'
labels:
  - area:accounts
milestone: m-3
dependencies: []
priority: high
ordinal: 16390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-15 (replaces decision-2), amended by decision-16: developers adopt one tool more readily than two, and alz already knows which client a repository belongs to. alz chooses the account folder per repository; it never reads, copies or prints logins. Design: LAYERING-SPEC §7.13.
Boundaries:
- Off by default: with nothing configured, nothing changes.
- alz sets only each target's account variable (Claude CLAUDE_CONFIG_DIR, Codex CODEX_HOME), defined by the target's adapter.
- A client layer may name the account its repositories expect, like commit_identity (decision-13); it applies only after a person runs alz account allow in that repository (alz account allow --remove withdraws it). The owner's override is machine-local user config, never committed.
- A named account that cannot apply fails closed (decision-16, owner decisions 7 and 8 in TASK-50.1): alz run and alz use fail before starting; the shell hook warns and always sets the variable, to the missing named folder, to the recorded account on a cache miss while the recorded source is still in the stack, or to a blocked folder that never exists (~/.claude-alz-blocked, ~/.codex-alz-blocked) when the declaration changed or the recorded source left the stack. A tool never starts on the default account or on a newly declared one instead.
- A value set outside alz (one that differs from alz's marker, for example from mise or direnv) is left alone with a warning instead of being overridden.
Out of scope: credentials and logins; the owner's dotfiles (claude-as, client-init), which move over separately once this ships.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every subtask is Done
- [ ] #2 After alz account allow in each repository, and with the shell hook installed, plain 'claude' and 'codex' in two repositories of two different clients start on the two accounts their layers name, and in a repository with no account configured they start on the default account (recorded end-to-end run)
- [ ] #3 A test proves alz never opens a file inside an account folder: it lists names only
<!-- AC:END -->
