---
id: TASK-131
title: Make the README a guide for people who use alz
status: In Progress
assignee:
  - '@claude'
created_date: '2026-11-28T02:59:00Z'
labels:
  - area:docs
milestone: m-1
dependencies:
  - TASK-7
priority: medium
type: docs
ordinal: 232390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The README mixes user and developer content and has four errors: alz:build is said to need the personal layer next to the clone (it comes from GitHub, cached); the examples pin @v1.0.0 while the personal layer's newest tag is v0.5.0; Status calls account switching planned although alz account which and allow exist; the 0.1.0 section covers only the registry install, not a from-source one. Developer content moves to CONTRIBUTING.md (TASK-7). Out of scope: release binaries and Homebrew, which do not exist yet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 README sections serve a user: what it is, install (from source), update, uninstall, moving from 0.1.0, quick start, layers, commands, safety, status; the Development section is replaced by a link to CONTRIBUTING.md (read the file)
- [ ] #2 Every command and option in the README exists in the CLI today (alz --help and each subcommand's --help)
- [ ] #3 The quick start runs as written in a scratch repository
- [ ] #4 The four errors in the description are fixed (read the file)
<!-- AC:END -->
