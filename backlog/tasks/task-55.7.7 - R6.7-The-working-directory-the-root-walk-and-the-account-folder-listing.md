---
id: TASK-55.7.7
title: 'R6.7: The working directory, the root walk and the account folder listing'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T10:11:00Z'
labels:
  - area:rust
  - area:accounts
  - area:security
milestone: m-7
dependencies:
  - TASK-55.7.6
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 166390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.13 "Paths", §9.1, decision-19, TASK-52).
- account/place.rs: WorkingDirectory::resolve(self, &AccountFolders) -> Place consumes the captured directory, so it is resolved exactly once; the R3.5 component walk checks each prefix before the next component; a directory that names an account folder as written is refused before any call. Place = InAccountFolder | InConfiguredFolder(real) | AtOrAboveHome | Directory(real).
- account/root_walk.rs: the nearest layer.yaml with a stack key; a layer's own manifest is skipped; one that cannot be used (link, folder, over 1 MiB, unreadable, invalid YAML, alias limit, schema) stops the walk with one warning naming the file; never a layer.yaml at or above HOME or in an account folder.
- account/listing.rs: one listing of $HOME: a folder or link named .claude-<name> or .codex-<name> exists; alz-blocked never counts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The working directory is resolved exactly once per run on macOS and Linux: an alz-io test counts the lookups through the walk's probe seam, and the resolver takes the Place by value (cargo test)
- [ ] #2 Property: neither walk lstats, reads a link of or opens a name inside an account folder, even through a link planted in the repository (proptest; TASK-52 #4, account side)
- [ ] #3 The root walk stops at each kind of unusable layer.yaml with one warning naming the file and never goes past it (integration tests)
<!-- AC:END -->
