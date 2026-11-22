---
id: TASK-57
title: 'Get started fast: starter layer, guided alz init and local layer editing'
status: To Do
assignee: []
created_date: '2026-11-22T14:00:00Z'
labels:
  - area:cli
  - area:process
milestone: m-3
dependencies: []
priority: high
ordinal: 50390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A new developer should go from nothing to a working, reviewable agent setup in a few minutes, without inheriting the author's identity or preferences, and keep full control over which layers she uses.

Decided by the owner (2026-11-22): alz offers a separate, generic public starter layer by default, never the author's personal layer. The author's personal layer stacks on top of the starter (starter -> personal -> client -> repo).

Why not the author's personal layer as the default: it sets the author's commit_identity, carries gr- skills and personal standards, lists client names in forbid_patterns (which would leak when published, TASK-12), and ships hooks, permissions and MCP servers that run on the user's machine, so following it unpinned is a supply-chain risk.

Parts: the spec for the workflow; the starter layer split; publish readiness of the author's layers; alz init with three paths and a trust preview; alz layer new --from; a machine-local default stack; local overrides for editing a pinned layer; docs and a runnable example. Code parts are built in Rust after the cutover (decision-21, decision-22).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every subtask is Done
- [ ] #2 On a clean machine, a developer runs alz init, chooses the starter, sees what it brings, confirms, and gets a building repository with no author identity in any generated file (recorded end-to-end run)
<!-- AC:END -->
