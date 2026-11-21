---
id: TASK-32
title: 'alz init --kind hub: scaffold a client hub'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-20T16:59:00Z'
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
