---
id: TASK-55.8.2
title: >-
  Build the four release binaries and publish a GitHub release from a tag, in CI
  only
status: To Do
assignee:
  - '@release-engineer'
created_date: '2026-11-24T17:44:00Z'
labels:
  - area:release
  - area:ci
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.8.1
parent_task_id: TASK-55.8
priority: medium
ordinal: 195390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-39 and decision-25: alz ships as prebuilt binaries for aarch64-apple-darwin, x86_64-apple-darwin and static x86_64- and aarch64-unknown-linux-musl, published as a GitHub release by CI only; decision-4's CI-only rule carries over without npm. Before relying on them, verify and record (docs/target-facts.md or a decision) how musl is built (for example cargo-zigbuild or cross), each target's current Rust support tier, and the runner images used. Untrusted input: the tag name and the tagged commit's content. The release job runs only for tags v<semver> whose version equals crates/alz's, is the only job with contents: write, and uses no long-lived token. Out of scope: Homebrew and the first real release.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A pushed tag v<version> builds the four targets with --locked in CI and refuses a tag whose version differs from crates/alz's (a run and a mismatched tag failing, in notes)
- [ ] #2 Each release archive has a SHA-256 sum and a GitHub artifact attestation, and the Linux binaries are statically linked (file and gh attestation verify output in notes)
- [ ] #3 The release is created only after the owner approves the release environment, actions are pinned to commit SHAs, and mise run workflows:lint passes
- [ ] #4 A pre-release tag produces a GitHub pre-release whose binaries print their version on macOS arm64 and Linux x86_64 (runs in notes)
<!-- AC:END -->
