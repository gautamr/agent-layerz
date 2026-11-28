---
id: TASK-131
title: Make the README a guide for people who use alz
status: In Progress
assignee:
  - '@claude'
created_date: '2026-11-28T02:59:00Z'
updated_date: '2026-11-28T03:07:00Z'
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
- [x] #1 README sections serve a user: what it is, install (from source), update, uninstall, moving from 0.1.0, quick start, layers, commands, safety, status; the Development section is replaced by a link to CONTRIBUTING.md (read the file)
- [x] #2 Every command and option in the README exists in the CLI today (alz --help and each subcommand's --help)
- [x] #3 The quick start runs as written in a scratch repository
- [x] #4 The four errors in the description are fixed (read the file)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Checks: every command and option checked against alz --help and each subcommand's --help (doctor and lint rows reworded to match, the doctor row against a real alz doctor run). The quick start ran in a scratch repository with a real tagged public layer: init, build --adopt (54 files), ls and check --skip-identity passed. That run showed a layer's commit_identity becomes the scratch repository's git identity, so the README no longer names anyone's layer as one to try, and it says what a layer can set. alz init --kind layer scaffolded a layer.
Review (fresh-context reviewer) found and fixed: the 0.1.0 section could not tell the two installs apart because the Rust alz also reports 0.1.0 (it now tells them apart by symlink or regular file), and it said layers keep working when CHANGELOG says some fail; CHANGELOG cited the old heading; the cache path left out XDG_CACHE_HOME; the garbled settings bullet in Safety is now a list; 'the clone now fails' is now timeless.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The README now serves people who use alz: install from source, update, uninstall (binary, layer cache, account state), moving from 0.1.0 by install kind, the quick start with a placeholder layer and a note on what a layer can set, corrected doctor, lint and status text; the Development section is replaced by a link to CONTRIBUTING.md. The four errors from the description are fixed. Reviewed in a fresh context; findings fixed. Depends on TASK-7 (CONTRIBUTING.md).
<!-- SECTION:FINAL_SUMMARY:END -->
