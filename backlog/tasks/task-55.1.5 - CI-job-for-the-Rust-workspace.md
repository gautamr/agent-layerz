---
id: TASK-55.1.5
title: CI job for the Rust workspace
status: In Progress
assignee:
  - '@release-engineer'
created_date: '2026-11-22T15:53:00Z'
updated_date: '2026-11-22T16:56:00Z'
labels:
  - area:ci
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.1.4
parent_task_id: TASK-55.1
priority: medium
ordinal: 63390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A GitHub Actions workflow for the Rust workspace on Linux and macOS: fmt check, clippy -D warnings, tests and doc tests, docs, cargo deny, and the MSRV job; pinned action versions; no secrets needed. It runs next to the TypeScript CI until cutover.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The workflow passes on a pull request that only touches crates/, and fails when clippy or rustfmt would fail
- [x] #2 Every third-party action is pinned to a full commit SHA
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. One workflow, .github/workflows/rust.yml, Rust only (no dependency on the TypeScript CI), on pull_request and push to main. Jobs: rust (ubuntu and macOS: rust:check, rust:doc), msrv (rust:msrv), supply chain (Linux: rust:deny, rust:audit). Same mise tasks as local, so mise.toml is the single source of the toolchain.
2. Pin every action to a full commit SHA looked up from the real tags; least privilege (contents: read, no secrets, persist-credentials: false), PR concurrency cancel, timeout per job.
3. Pin actionlint and zizmor in mise.toml with a workflows:lint task; lint the workflow; run mise run rust:ci.
4. Record how to prove AC #1 on a real run.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented (not pushed, not committed; AC #1 needs a real GitHub run).

Files: .github/workflows/rust.yml (new); mise.toml (adds pinned actionlint 1.7.12 and zizmor 1.32.0 and a workflows:lint task).

Jobs: rust (ubuntu-latest and macos-latest, fail-fast off: mise run rust:check, then rust:doc), msrv (ubuntu: mise run rust:msrv), supply-chain (ubuntu: rust:deny, rust:audit). rust:check already runs the doc tests through cargo test --workspace, so rust:doctest is not repeated. cargo-audit runs on Linux only because its aqua release is built for Intel macOS only (checked: Mach-O x86_64); cargo-deny reads the lockfile and all targets, so one OS covers it.

Pinned actions (full commit SHA, tag): actions/checkout 3d3c42e5aac5ba805825da76410c181273ba90b1 (v7.0.1); jdx/mise-action 924e7c586e33d7d4bae9c14e26c5d22e2beb6e78 (v5.3.0); Swatinem/rust-cache d1527c8f67ef9af357290475d4af4b0fab8f12e0 (v2.9.3). The rust-cache v2.9.3 tag is annotated: the tag object is 2201cc58..., the commit to pin is d1527c8f.... SHAs came from git ls-remote and gh api, and each was confirmed to be a commit.

Choices and why:
- No paths: filter. GitHub keeps the checks of a workflow skipped by path filtering "Pending" and blocks merge when they are required (docs: troubleshooting required status checks), while a job skipped by a conditional reports Success. The workspace is small, so the workflow runs on every PR. If cost becomes a problem, use an always-running changes job plus job-level if, not workflow-level paths.
- Required check names to configure on the branch rule: "rust (ubuntu-latest)", "rust (macos-latest)", "msrv", "supply chain".
- Caching: Swatinem/rust-cache for dependency builds, saved only on main (PRs restore main's cache, never write). mise-action cache is off: mise's Rust is a symlink into rustup's directory, which mise's cache does not hold; the pinned tools are small downloads.
- MISE_AUTO_INSTALL=false: verified locally that mise run installs every missing tool in mise.toml by default, which would defeat install_args (Node, linters in each job). Each job installs only what it needs.
- Concurrency cancels in-progress runs for pull requests only; never on main. Timeouts 30/20/15 minutes. Top-level permissions contents: read; no job needs more, so no id-token anywhere (the release workflow is separate work).
- Not added: scheduled run for new advisories. The supply-chain job only sees a new advisory when some PR or push runs it; a weekly cron would catch it sooner. Suggested follow-up.
- mise-action v5.x persists its github_token (the read-only workflow token) to later steps; accepted because the permissions are contents: read.

Local validation: actionlint 1.7.12 clean; zizmor 1.32.0 --pedantic clean, offline and online; zizmor flags an unpinned action in a negative copy; mise run rust:ci passes (fmt, clippy, tests, doc, msrv, deny, audit). In a scratch copy, a badly formatted line made rust:check fail on cargo fmt, and a clippy::clone_on_copy made it fail on clippy; both passed again once restored.

Cannot be proved locally: that jdx/mise-action installs the rustup toolchain with rustfmt and clippy on the hosted runners, that install_args resolves the aqua tool names there, that cargo-audit runs on ubuntu-latest, and AC #1.

Verify AC #1 (needs the owner): 
1. Push the branch and open a PR that touches only crates/ (for example a comment in crates/alz-core/src/lib.rs). Expect rust (ubuntu-latest), rust (macos-latest), msrv and supply chain to pass.
2. Add a throwaway commit that breaks rustfmt (append `pub   fn   badly_formatted ( )  {  }` to crates/alz-core/src/lib.rs). Expect both rust jobs to fail in "Format, clippy and tests" with a cargo fmt diff.
3. Replace it with a commit that is formatted but fails clippy (a `pub fn` with `let x = 1; x.clone()` returning i32, with a doc comment and #[must_use]). Expect both rust jobs to fail on clippy::clone_on_copy.
4. Drop the throwaway commits; the PR passes again. AC #2 can be checked by reading the uses: lines (all are 40-hex SHAs with a version comment; zizmor's unpinned-uses audit passes).

Orchestrator: all eight uses: lines are 40-hex SHAs with version comments; mise run workflows:lint (actionlint 1.7.12, zizmor 1.32.0 pedantic) exit 0. AC #1 waits for a real GitHub run (needs the owner's go-ahead to push).
<!-- SECTION:NOTES:END -->
