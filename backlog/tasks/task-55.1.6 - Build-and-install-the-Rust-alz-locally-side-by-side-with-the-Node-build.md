---
id: TASK-55.1.6
title: Build and install the Rust alz locally, side by side with the Node build
status: Done
assignee:
  - '@claude'
created_date: '2026-11-22T17:31:00Z'
updated_date: '2026-11-22T17:32:00Z'
labels:
  - area:rust
  - area:cli
milestone: m-7
dependencies: []
parent_task_id: TASK-55.1
priority: medium
ordinal: 70390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner request: a local build, install and update path for the Rust binary like install:local has for the Node bundle. Until cutover the Rust alz only prints its version and help, so it installs as alz-rs by default and never replaces the Node alz unless asked.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 mise run rust:build builds the release binary with --locked
- [x] #2 mise run install:local:rust installs the release binary as one regular file (default ~/.local/bin/alz-rs; PREFIX, BIN_DIR and ALZ_NAME override), refuses a symlink or non-regular target, replaces it atomically, checks its SHA-256 against the build and prints version, git revision and hash
- [x] #3 Rerunning install:local:rust updates an existing install in place; installing over a Node alz needs ALZ_NAME=alz and says what it replaced
- [x] #4 The command table in .layer lists both tasks and the generated AGENTS.md is current
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. scripts/install-local-rust.sh modelled on install-local.sh (same PREFIX/BIN_DIR, staged copy + mv, hash check, symlink refusal). 2. mise tasks rust:build and install:local:rust. 3. .layer command table rows, alz:build. 4. Verify: fresh install, rerun update, symlink refusal, ALZ_NAME=alz into a scratch BIN_DIR over a copy of the Node bundle; rust:ci, workflows:lint, alz:check.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified in a scratch BIN_DIR: fresh install; rerun updates in place (prints the previous version); a symlink target and ALZ_NAME=../x are refused; ALZ_NAME=alz over a copy of the Node bundle replaces it and reports 'previous the Node build of alz'; SHA-256 of the installed file matches the build. shellcheck 0.11.0 clean. mise run rust:build, alz:build (AGENTS.md updated), alz:check and lint pass. Installed the owner's ~/.local/bin/alz-rs: 'alz 0.1.0'; the Node ~/.local/bin/alz is untouched and prints '0.1.0'.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added scripts/install-local-rust.sh and the mise tasks rust:build and install:local:rust. The Rust alz builds with --locked and installs as one regular file, alz-rs by default (PREFIX, BIN_DIR, ALZ_NAME override), next to the Node alz until cutover; rerunning it is the update path. It refuses symlinks, non-regular targets and names with a slash, replaces atomically, checks the SHA-256 and prints version, git revision, hash and what it replaced. Command table updated through .layer. Follow-up at cutover (TASK-55.9): make alz the default name and retire install-local.sh.
<!-- SECTION:FINAL_SUMMARY:END -->
