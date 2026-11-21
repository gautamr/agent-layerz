---
id: TASK-21
title: 'alz publish: lint a layer and tag its release'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-19T17:14:00Z'
labels:
  - area:cli
milestone: m-3
dependencies: []
priority: low
ordinal: 330
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The spec lists alz publish, but the CLI does not have it; layer releases are tagged by hand.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Runs alz lint, including the leak guard, and refuses on failure
- [ ] #2 Tags v<version> from layer.yaml, or layer-v<version> for a hub's client layer, and pushes only with --push
- [ ] #3 Spec and README are updated
<!-- AC:END -->
