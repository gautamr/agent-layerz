---
id: TASK-55.8.1
title: Create the release environment and the Homebrew tap repository
status: To Do
assignee: []
created_date: '2026-11-24T17:44:00Z'
labels:
  - owner:human
  - area:release
milestone: m-7
dependencies: []
parent_task_id: TASK-55.8
priority: medium
ordinal: 194390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Only the owner can change repository settings and create repositories (delivery-protocol §4). R7 needs a GitHub environment named release with the owner as required reviewer and deployments limited to v* tags; a tag ruleset so only the owner creates v* tags; a public tap repository for Homebrew (name to choose, for example homebrew-tap); and the credential the release workflow uses to push the formula there, scoped to that repository only (a GitHub App or a fine-grained token), stored as a secret of the release environment. Needs the public repository and its settings (TASK-2, TASK-5).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The release environment exists with the owner as required reviewer and only v* tags allowed (owner comment)
- [ ] #2 Only the owner can create v* tags (tag ruleset, owner comment)
- [ ] #3 The tap repository exists, and the release environment holds a credential that can write only to it (owner comment)
<!-- AC:END -->
