---
id: decision-15
title: >-
  alz optionally chooses each repository's account folder (a shell hook and alz
  run, account named by a layer, machine-local override) and never reads logins;
  replaces decision-2
date: '2026-11-21T17:57:00Z'
status: accepted
---
## Context

decision-2 kept multi-account handling in a separate project. Developers adopt one tool more
readily than two, and alz already knows which client a repository belongs to from its layers.
Claude Code and Codex each pick their account folder from one variable (`CLAUDE_CONFIG_DIR`,
`CODEX_HOME`).

## Decision

alz optionally chooses the account folder per repository: `alz run`, `alz use` and a shell hook
set only each target's account variable. A client layer may name the account its repositories
expect, under the same who-may-declare rule as `commit_identity` (decision-13). The owner's
override lives in machine-local user config, never committed. alz never reads, copies or prints
logins or credentials. Off by default: with nothing configured, nothing changes.

## Consequences

LAYERING-SPEC gains goal 8 and §7.13; the non-goal narrows to credentials and logins. The work
is TASK-50 and its subtasks. decision-16 and decision-17 refine it; decision-16 replaces its
original fallback to the default account with failing closed.
