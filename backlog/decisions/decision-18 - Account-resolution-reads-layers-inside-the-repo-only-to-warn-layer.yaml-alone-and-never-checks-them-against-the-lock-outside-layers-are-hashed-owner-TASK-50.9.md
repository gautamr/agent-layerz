---
id: decision-18
title: >-
  Account resolution reads layers inside the repo only to warn (layer.yaml
  alone) and never checks them against the lock; outside layers are hashed
  (owner, TASK-50.9)
date: '2026-11-22T07:23:00Z'
status: accepted
---
## Context

LAYERING-SPEC §7.13 checked every layer resolution reads against the lock's content hash,
including the repository's own unpinned `path:./.layer`. In an allowed repository any edit under
`.layer/` since the last `alz build` would then be a cache miss and block the account. A layer
inside the repository can never declare an account, so its content cannot change the result.

## Decision

Account resolution reads layers inside the repository only to warn when one declares an account
(their `layer.yaml` alone) and never checks them against the lock; such a declaration is reported
as ignored and never causes a conflict. Layers outside the repository are hashed and checked.

## Consequences

Editing `.layer/` never blocks the account. Stack validation during resolution covers only the
layers outside the repository. Implemented in TASK-50.9.
