---
id: TASK-55.1.4
title: Supply-chain and quality tooling for the Rust workspace
status: To Do
assignee: []
created_date: '2026-11-22T15:53:00Z'
updated_date: '2026-11-22T15:53:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.1.3
parent_task_id: TASK-55.1
priority: medium
ordinal: 62390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add deny.toml (licences, bans including duplicate versions, advisories, sources) and mise tasks for cargo deny, cargo audit, docs with -D warnings, doc tests, and an MSRV check (cargo hack --rust-version or a pinned toolchain). Optional: cargo-nextest. Record any tool added as a dev dependency decision if it is a crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 mise tasks run cargo deny check, cargo audit, the docs build with -D warnings and the MSRV check, all passing on the skeleton
- [ ] #2 rust:check (or a new rust:ci task) runs the full local set in one command
<!-- AC:END -->
