---
id: TASK-32
title: 'alz init --kind hub: scaffold a client hub'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-20T16:59:00Z'
updated_date: '2026-11-25T18:55:00Z'
labels:
  - area:cli
milestone: m-3
dependencies: []
documentation:
  - backlog/docs/doc-4 - Onboarding-projects-greenfield-and-existing.md
priority: medium
ordinal: 318
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Greenfield multi-repository clients need a hub before their repositories; today it takes several manual steps (doc-4).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz init --kind hub creates the hub layout with a private client layer in layer/ and the hub's own stack manifest
- [ ] #2 A later alz init --hub in a sibling repository finds the client layer
- [ ] #3 Spec, README and doc-4 are updated
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Inputs from the earlier setup (retired; archive a local archive): the hub profile had contracts/README.md (contracts change in the hub first, producers implement, consumers generate), repos.conf (name, git URL, label) and scripts/workspace.sh (clone, status, pull, check across all repos; accounts moved to the separate accounts project per decision-2). Consider them for the hub scaffold.
<!-- SECTION:NOTES:END -->
