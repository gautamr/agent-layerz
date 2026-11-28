---
id: TASK-55.8.5
title: Take agent-layerz off npm after the Rust release
status: To Do
assignee: []
created_date: '2026-11-24T17:45:00Z'
updated_date: '2026-11-27T06:36:00Z'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-24): unpublish the npm package agent-layerz rather than deprecate it, accepting that the name may be claimed later. The owner does it.

2026-11-27, owner in session: "I deprecated agent-layerz in NPM console". So the name is kept, not given up, which replaces the 2026-11-24 choice to unpublish. npm view agent-layerz shows both versions (0.0.0-stage, 0.1.0) deprecated with npm's default message, 'Package no longer supported. Contact Support at https://www.npmjs.com/support for more info.' AC #1 also wants the message to point to the GitHub releases and Homebrew, so it is reset with npm deprecate after 0.2.0 (TASK-55.8.4). The local TypeScript alz (~/.local/bin/alz) was removed the same day.
<!-- SECTION:NOTES:END -->
