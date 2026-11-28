---
id: TASK-7
title: Write CONTRIBUTING.md
status: In Progress
assignee:
  - '@claude'
created_date: '2026-11-19T17:13:00Z'
updated_date: '2026-11-28T15:38:00Z'
labels:
  - area:docs
milestone: m-1
dependencies: []
priority: medium
ordinal: 200
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Contributors need the setup, the test conventions and the rules this project holds itself to, including that its own agent configuration is generated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Covers setup with mise and the test, lint and build commands
- [x] #2 Explains the tests and the acceptance fixtures (spec §13, fixtures/ and the golden trees), pointing to the spec-test skill
- [x] #3 States Conventional Commits with the task ID and one task per pull request
- [x] #4 States that adapter behaviour must rest on a recorded fact in docs/target-facts.md
- [x] #5 Explains that AGENTS.md, CLAUDE.md and .claude/ are generated from .layer/ with `mise run alz:build`
- [ ] #6 docs/algebra.md states each merge structure, its laws, what each law gives and the property test that checks it, with further reading (read the file; every named test exists)
- [ ] #7 CONTRIBUTING shows the algebra with a small example that compiles and passes, and invites adapters for other coding agents
- [ ] #8 A pull request template and issue forms for a bug, a feature request and an adapter exist under .github/
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Write CONTRIBUTING.md: setup through mise; using and building at the same time (stable alz from install:local, a working copy as alz-dev, cargo run); the checks and what CI runs; dogfooding (generated files come from .layer/); the code map; spec, tests (spec-test skill, fixtures/) and target facts; the board, one task per PR, Conventional Commits, the commit identity, releases from CI only.
2. Check every command in it by running it or reading mise.toml.
3. Fresh-context review.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Review (fresh-context reviewer) found: the CI claim was wrong (three jobs run on Linux only, and alz:check and workflows:lint are outside rust:ci); the Commits section of AGENTS.md is the owner's and conflicted with 'commit under your own name'; expected output lives in crates/alz/tests/fixtures/golden/, not fixtures/; cargo run fails outside the clone because mise puts cargo on PATH only there. All fixed. Also added rust:build, spec-coverage, decisions, the dependency rule, mise trust, and the XDG cache path. AC #2 now names the spec-test skill (the add-acceptance-test skill no longer exists).
Checks: ALZ_NAME=alz-dev install:local into a scratch BIN_DIR installed and ran; target/release/alz check --skip-identity from a scratch repository passed; every mise task, env var and CI job cited was read in mise.toml and .github/workflows/rust.yml.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
CONTRIBUTING.md for people who change agent-layerz: setup through mise; using alz while changing it (stable alz, an alz-dev copy, the release binary); the checks and which CI job runs which; how the repository builds itself from .layer/; the code map; the spec, tests, acceptance fixtures and target-facts rules; the board, one task per PR, Conventional Commits and commit identity. Reviewed in a fresh context; findings fixed. Open: no SECURITY.md yet (TASK-6).
<!-- SECTION:FINAL_SUMMARY:END -->
