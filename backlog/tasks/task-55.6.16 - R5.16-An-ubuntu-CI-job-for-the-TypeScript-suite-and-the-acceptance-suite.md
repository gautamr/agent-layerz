---
id: TASK-55.6.16
title: 'R5.16: An ubuntu CI job for the TypeScript suite and the acceptance suite'
status: Done
assignee:
  - '@release-engineer'
created_date: '2026-11-23T18:16:00Z'
updated_date: '2026-11-24T06:22:00Z'
labels:
  - area:ci
  - area:rust
milestone: m-7
dependencies: []
parent_task_id: TASK-55.6
priority: high
ordinal: 152390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner (2026-11-23): add an ubuntu job to .github/workflows that runs mise run test (the TypeScript reference and the acceptance suite with ALZ_BIN unset), so Linux-only acceptance cases (0xFF file names, TASK-69 AC #5) run in CI; and, once R5's commands exist, a second step that runs the acceptance project against ALZ_BIN=target/release/alz. Actions pinned to SHAs, mise pinned with its sha256, MISE_LOCKED=1, least-privilege permissions, actionlint and zizmor clean (mise run workflows:lint).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An ubuntu CI job runs mise run test on every pull request, and TASK-69's Linux acceptance cases run in it
- [x] #2 A step runs the acceptance project against the release Rust binary, with the R6 cases listed as expected to fail until R6
- [x] #3 mise run workflows:lint passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added .github/workflows/tests.yml: job 'typescript' (ubuntu-latest, mise node only, npm ci --ignore-scripts, npm cache restore on PRs and save on main, mise run test) and job 'acceptance-rust' (continue-on-error, node+rust, rust-cache, mise run rust:build, ALZ_BIN=$GITHUB_WORKSPACE/target/release/alz mise run test:acceptance; TASK-55.6.15 removes continue-on-error). Pins: mise 2026.9.1 + sha256 as rust.yml, MISE_LOCKED=1, contents: read only, actions/cache v6.1.0 pinned by SHA. Verified locally (macOS): actionlint (with real shellcheck 0.11.0) and zizmor --pedantic --offline clean; mise run lint clean; npm ci --ignore-scripts then mise run test: 50 files passed, 1186 passed, 188 expected fail, 5 skipped (the 5 are the Linux-only 0xFF cases in test 36, gated on process.platform === 'linux', so they run on ubuntu); ALZ_BIN=target/release/alz acceptance locally: 701 failed, 293 passed, exit 1 (the reason for continue-on-error). Not provable locally: the ubuntu run itself, cache save on main, that the 0xFF cases run as expected failures against the reference there.

Verified by the lead (2026-11-24): merged into the R5 branch (dfbe9e6); actionlint (with shellcheck 0.11.0) and zizmor find nothing. AC #2 is met by a non-blocking acceptance-rust job; R5.15 makes it blocking and lists the R6 cases.

Met in R5.15 (2026-11-24): acceptance-rust no longer has continue-on-error; it runs the acceptance project with the JSON reporter and test/ci/check-r6-pending.ts, which fails on any failure outside the listed R6 cases and on a listed case that passes.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
An ubuntu workflow runs mise run test (the TypeScript reference and the acceptance suite, Linux-only cases included) and a blocking acceptance run against the release Rust binary with the R6 cases listed.
<!-- SECTION:FINAL_SUMMARY:END -->
