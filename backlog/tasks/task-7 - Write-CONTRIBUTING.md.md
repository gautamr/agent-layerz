---
id: TASK-7
title: Write CONTRIBUTING.md
status: In Progress
assignee:
  - '@claude'
created_date: '2026-11-19T17:13:00Z'
updated_date: '2026-11-28T15:50:00Z'
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
- [x] #6 docs/algebra.md states each merge structure, its laws, what each law gives and the property test that checks it, with further reading (read the file; every named test exists)
- [x] #7 CONTRIBUTING shows the algebra with a small example that compiles and passes, and invites adapters for other coding agents
- [x] #8 A pull request template and issue forms for a bug, a feature request and an adapter exist under .github/
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

Owner feedback, 2026-11-28: the algebra with a small example, a call for adapters, and PR and feature request templates. Added docs/algebra.md, the CONTRIBUTING sections and .github templates. The CONTRIBUTING example compiled and passed as a throwaway test in crates/alz-core/tests (3 passed, file removed). The reading list was checked against publishers, DOIs and official pages.
Second fresh-context review, on the algebra: Floor is used only for default_mode with floor: true; locked hooks, floor flags, settings floors and overlay modes are refusals, each with its own property test, now in their own table. Instructions are not Replace and hook order is not OrderedSet. Native fields use NativeMap, and rules use RuleChain; both are now listed. Associativity is now stated per structure, not for whole layers, with the hook counterexample. Deep merge is a semigroup on trees of one shape, with the identity only at the top of settings.yaml and the twice-merged law. ListEdit's identity is tested up to apply. proptest runs 256 cases by default, not thousands. Commutativity holds only when layers only add. The example deny_lists_only_grow became a_union_keeps_every_earlier_rule, with the refusal explained. The bug form now gives a route for security reports. Not changed: the module doc of merge/settings/laws.rs says 'idempotent' without the duplicate-free condition (code comment, out of scope).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
CONTRIBUTING.md for people who change agent-layerz: setup through mise; using alz while changing it; the checks and which CI job runs which; how the repository builds itself from .layer/; the code map; the spec, tests, acceptance fixtures and target-facts rules; the algebra in practice with a tested example; how to write an adapter for another coding agent; the board, one task per PR, Conventional Commits and commit identity. docs/algebra.md states each merge structure, its laws at the level they are tested, the refusals that keep security monotone, what is not a single structure, and a verified reading list. .github has a PR template and issue forms for a bug, a feature request and an adapter. Two fresh-context reviews; findings fixed. Open: no SECURITY.md or private vulnerability reporting yet (TASK-6).
<!-- SECTION:FINAL_SUMMARY:END -->
