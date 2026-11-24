---
id: TASK-55.8.5
title: Take agent-layerz off npm after the Rust release
status: To Do
assignee: []
created_date: '2026-11-24T17:45:00Z'
labels:
  - owner:human
  - area:release
milestone: m-7
dependencies:
  - TASK-55.8.4
parent_task_id: TASK-55.8
priority: medium
ordinal: 198390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-39: no npm distribution; the owner removes the package. npm's unpublish policy (docs.npmjs.com/policies/unpublish, read 2026-11-24) allows removing a package older than 72 hours only if no other package depends on it, it had fewer than 300 downloads in the previous week and it has a single owner; a used version number can never be reused, and unpublishing every version blocks new versions for 24 hours. The page does not say whether another account can then claim the name. Deprecating instead keeps the name and shows every installer a message. Choose one after the first Rust release, so the message can point to the new install. Replaces TASK-3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 npm view agent-layerz shows no package, or a deprecation message pointing to the GitHub releases and Homebrew (owner comment with the output)
- [ ] #2 The owner records in a comment whether the name was kept (deprecated) or given up (unpublished)
<!-- AC:END -->
