---
id: TASK-50
title: Optional account switching in alz
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
labels:
  - area:accounts
milestone: m-3
dependencies: []
priority: high
ordinal: 16390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-15 (replaces decision-2): developers adopt one tool more readily than two, and alz already knows which client a repository belongs to. alz chooses the account folder per repository; it never reads, copies or prints logins.
Boundaries:
- Off by default: with nothing configured, nothing changes.
- alz sets only each target's account variable (Claude CLAUDE_CONFIG_DIR, Codex CODEX_HOME), defined by the target's adapter.
- A client layer may name the account its repositories expect, like commit_identity (decision-13). The owner's override is machine-local user config, never committed. A missing account folder falls back to the default account.
- When mise or direnv already sets the variable, alz reports the conflict instead of overriding it.
Out of scope: credentials and logins; the owner's dotfiles (claude-as, client-init), which move over separately once this ships.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every subtask is Done
- [ ] #2 With the shell hook installed, plain 'claude' and 'codex' in two repositories of two different clients start on the two accounts their layers name, and in a repository with no account configured they start on the default account (recorded end-to-end run)
- [ ] #3 A test proves alz never opens a file inside an account folder: it lists names only
<!-- AC:END -->
