---
id: TASK-43
title: >-
  Let a hub stack its own client layer when that layer declares a commit
  identity
status: To Do
assignee: []
created_date: '2026-11-21T13:52:00Z'
labels:
  - area:spec
  - area:security
dependencies:
  - TASK-34
priority: high
ordinal: 9390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
LAYERING-SPEC §14 step 2 tells a client hub to put `commit_identity` in `hub/layer/` when the client issues its own email, and §7.12 says that in the hub repository itself `layer/` is inside the repository, so the hub's own commits use the first layer's identity. The build does not do that: when the hub stacks `path:./layer` and that layer declares an identity, `alz build` fails with "cannot declare commit_identity: it is inside the repository" (src/merge/commit-identity.ts). A hub therefore cannot both publish a client identity to its sibling repositories and use its own client layer (its skills and overlays) for the engineering manager. Found 21 Nov while setting up a second client hub; the workaround is to drop the identity, which makes every sibling repository generate the wrong Commits rule.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 In a hub whose stack includes its own `kind: client` layer from inside the repository, a `commit_identity` declared there is ignored for that build, and the hub uses the first layer's identity as §7.12 says
- [ ] #2 The build reports which declaration it ignored and why, by layer name, without printing the values
- [ ] #3 A sibling repository stacking the same layer from outside still gets that layer's identity; the repo layer, group layers and a public `client` layer still fail as today
- [ ] #4 LAYERING-SPEC §7.12 states the rule without contradiction, and acceptance tests cover the hub and a sibling repository built from one client layer
<!-- AC:END -->
