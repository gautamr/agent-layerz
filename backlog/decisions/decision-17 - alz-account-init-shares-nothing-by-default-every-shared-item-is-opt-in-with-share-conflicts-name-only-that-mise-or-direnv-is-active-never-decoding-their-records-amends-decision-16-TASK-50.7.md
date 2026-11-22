---
id: decision-17
title: >-
  alz account init shares nothing by default; every shared item is opt-in with
  --share; conflicts name only that mise or direnv is active, never decoding
  their records; amends decision-16 (TASK-50.7)
date: '2026-11-22T02:43:00Z'
status: accepted
---
## Context

The verified facts (TASK-50.7, target-facts §12) showed that sharing whole `skills/` and
`plugins/` folders mixes claude.ai organisation items between accounts, that other shared items
carry account-specific state, and that mise and direnv record what they set in variables that
also hold secret values. The owner does not want symlinks unless asked.

## Decision

`alz account init` shares nothing by default: it only creates the account folders (mode 0700),
and every shared item is opt-in with `--share <item>` (Claude skills and plugins linked per entry,
`synced/` and `.trash/` per account). A value set outside alz is reported as "set outside alz",
adding "mise or direnv is active here" when `__MISE_DIFF` or `DIRENV_DIFF` exists; alz never reads
or decodes those variables.

## Consequences

Amends decision-16. Personal skills and agents reach alz-managed repositories through
`alz build` at project scope, so account folders need no shared items for them. alz cannot name
which tool set a variable.
