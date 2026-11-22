---
id: decision-19
title: >-
  Account resolution stops at a layer.yaml it cannot read on the root walk and
  fails closed when that root is allowed; never skips past it (owner,
  TASK-50.10)
date: '2026-11-22T08:20:00Z'
status: accepted
---
## Context

Account resolution finds the repository root by walking up from the working directory to the
nearest `layer.yaml` with a `stack` key. A manifest on the way can be unreadable: no permission,
over 1 MiB, invalid YAML or a link. Skipping past it would let resolution pick a root further up,
whose allowed account the folder may not belong to.

## Decision

The walk stops at a `layer.yaml` it cannot read, treats that folder as the root and warns. When
that root is in `allowed.yaml`, every target the entry names fails closed with the blocked folder;
otherwise the default account applies. The walk never skips past an unreadable manifest.

## Consequences

A broken manifest in an approved repository starts the tools logged out until it is fixed.
Implemented in TASK-50.10 (`src/resolve/account-root.ts`), with tests for each kind of unreadable
file in test 31.
