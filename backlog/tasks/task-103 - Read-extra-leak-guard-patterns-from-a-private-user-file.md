---
id: TASK-103
title: Read extra leak-guard patterns from a private user file
status: To Do
assignee: []
created_date: '2026-11-25T18:56:00Z'
labels:
  - area:security
  - area:rust
  - needs:owner-approval
milestone: m-7
dependencies: []
priority: medium
ordinal: 205390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner (2026-11-26): forbid patterns that name clients must never sit in a public layer, because the list itself would publish the names. alz reads extra forbid patterns from a private file in the user's config (e.g. ${XDG_CONFIG_HOME:-~/.config}/agent-layerz/forbid.yaml, outside every repository and account folder, owned by the user and not writable by others, like accounts.yaml) and applies them to every public layer it lints or builds, never printing a pattern or a match.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The spec states the file, its checks and that its patterns apply to every public layer (owner approved)
- [ ] #2 alz lint and alz build refuse a public layer that matches a pattern of the file, naming the layer, the file and the pattern's number only (Rust test through the binary)
- [ ] #3 A file inside a repository, an account folder or writable by others is ignored with a warning (Rust test)
<!-- AC:END -->
