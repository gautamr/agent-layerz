---
id: TASK-13
title: 'Publish the personal layer and stack it through github:'
status: To Do
assignee:
  - '@gautamr'
created_date: '2026-11-19T17:13:00Z'
updated_date: '2026-11-25T18:56:00Z'
labels:
  - owner:human
  - area:repo
milestone: m-1
dependencies:
  - TASK-57.3
priority: medium
ordinal: 250
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
This repository stacks the personal layer as path:../agent-layerz-personal, which contributors and CI cannot resolve.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The personal layer contains no client or private names
- [x] #2 The personal layer is public on GitHub with a v0.1.0 or later tag
- [x] #3 This repository's layer.yaml uses github:gautamr/agent-layerz-personal@<tag>
- [ ] #4 alz check passes in a fresh clone
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22: publishing now follows TASK-57.3 (publish readiness) and covers both agent-layer-starter and agent-layer-personal; the starter is the layer alz offers by default.

2026-11-22: the personal layer folder and repository are renamed agent-layerz-personal, and the planned starter is agent-layerz-starter; earlier notes keep the old names.

Lead (2026-11-26, owner asked): the personal layer is public at github.com/gautamr/agent-layerz-personal as one fresh commit db9c037 tagged v0.5.0, authored by the noreply identity. Its earlier local history held a leak-guard list naming clients and an earlier author address, so (owner's choice) it was not pushed; it stays in a local backup branch. The names list left the layer (owner: move to a private user file, TASK below). Provenance notes about the retired setup were removed. This repository stacks github:gautamr/agent-layerz-personal@v0.5.0; alz build regenerated 32 files (provenance labels only). AC #4 is proven by a fresh clone and CI's agent-config job.
<!-- SECTION:NOTES:END -->
