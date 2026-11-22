---
id: TASK-55.1
title: 'R0: Rust foundation and recorded choices'
status: Done
assignee: []
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-22T17:41:00Z'
labels:
  - area:rust
  - area:spec
milestone: m-7
dependencies: []
references:
  - decision-23
  - decision-24
  - decision-25
  - decision-26
  - decision-27
  - decision-28
parent_task_id: TASK-55
priority: high
ordinal: 33390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R0: repository layout, toolchain through mise, the YAML library spike, git access, MSRV and targets, output-text policy. Each choice becomes a decision.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Cargo workspace builds an alz binary that prints its version and help; the five choices in doc-5 are recorded as decisions
- [x] #2 The Rust standard in doc-5 is written as a .layer rule for Rust code and enforced in CI from the first commit: rustfmt, clippy -D warnings with pedantic and the chosen restriction lints, forbid(unsafe_code), cargo deny and cargo audit
- [x] #3 The crate layout is designed from the spec as Rust and recorded as a decision before any domain code; no module mirrors src/ because src/ had it
- [x] #4 Every R0 subtask is Done
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22: proposed layout, crate split and type-level design are in doc-6; the Rust standard is the personal skill gr-rust (agent-layer-personal, uncommitted until the owner releases it). R0 records the layout as a decision, adopting or changing doc-6.

2026-11-22: the personal layer folder and repository are renamed agent-layerz-personal, and the planned starter is agent-layerz-starter; earlier notes keep the old names.

2026-11-22: split into small subtasks (spike, decisions, skeleton, tooling, CI). The .layer Rust rule, Rust-aware agents, rust:check and the cargo stop hook were done in commit 23ca45e, which covers the rule half of AC #2; CI enforcement is the CI subtask.

2026-11-22: R0 choices 1, 3, 4, 5 and the directory-access crate are recorded as decision-23 to decision-27 (TASK-55.1.2). The YAML library (TASK-55.1.1) is still open.

2026-11-22: the YAML library (doc-5 choice 2) is recorded as decision-28 (TASK-55.1.1). All six R0 choices are now decisions: decision-23 to decision-28.

2026-11-22: R0 review fixes (architect). The owner approved amending four accepted decisions with tightenings from the security review; each carries an "Amended 2026-11-22 (owner, R0 review)" note in its body, titles and statuses unchanged.
- decision-24: the child environment is built explicitly (env cleared, one pure function of the Env snapshot and the operation kind); for cache and clone operations R3 removes GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE, GIT_OBJECT_DIRECTORY, GIT_ALTERNATE_OBJECT_DIRECTORIES, GIT_COMMON_DIR, GIT_NAMESPACE, GIT_CEILING_DIRECTORIES (alz check as a pre-commit hook). Revisions follow --end-of-options; -- only before paths. SMOKE on git 2.50.1: a pre-commit hook gets GIT_INDEX_FILE, and GIT_DIR in a linked worktree; checkout -- <sha> fails as a pathspec. GIT_CONFIG_PARAMETERS stays open for R3; identity calls go to R6.
- decision-26: three text classes (stripped for diagnostics, line feed included; escaped for text a person decides on; clap's output keeps only its own line feeds, argument fragments stripped); format = all of Unicode Cf; a panic hook prints a fixed message and exits 70.
- decision-27: every trust check uses fstat on the handle that is read; files opened O_NONBLOCK (O_NOFOLLOW where links are forbidden), S_ISREG required, the 1 MiB cap enforced by reading at most cap + 1 bytes; the R5 default refuses writes through links inside the repository.
- decision-28: tags outside the YAML 1.2 core set are errors with their own code; several documents are an error; decoded strings are checked for control characters; key names in errors are stripped per decision-26 class 1; caps and budgets cover every YAML file, accounts.yaml and allowed.yaml included. The '!!binary decoding unverified' consequence is removed. Found: serde-saphyr 2.2.0 drops tags in deserialize_any for untyped visitors, so R2 must choose how tags reach alz (reject_unsupported_tags, Tagged<T> or a granit_parser pre-pass; UNVERIFIED).
- doc-5 and doc-6 now match Cargo.toml (lints, no expect_used, lints through [workspace.lints]), decision-27's wording, the mise-only toolchain (no rust-toolchain.toml), the move of sanitising into alz-core in R2, and the amendments.

2026-11-22 review: reviewer and security-reviewer ran on 5ff9c97..fc4f1e5; no blockers or high findings. Fixes in 8132f5b (binary, tooling), 37e37ec (CI), dfaaa11 (owner-approved amendments to decisions 24/26/27/28, doc-5/doc-6). Discovered: TASK-59 to TASK-63. Open: AC #2 and #4 wait on TASK-55.1.5 AC #1, which needs a GitHub run; the owner chose not to push yet. Orchestrator verified after fixes: mise run rust:ci exit 0, workflows:lint clean, alz:check pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
R0 is complete. Decisions 23 to 29 record layout, YAML, git, toolchain and targets, output ownership, directory access and the first dependencies; 24, 26, 27 and 28 were tightened after the security review with owner approval. The Cargo workspace (four crates, Rust 1.100.0 through mise) builds alz with --version and --help; the Rust standard is enforced locally (rust:ci) and in CI, which passed on PR #1 and failed on throwaway rustfmt and clippy PRs. install:local:rust installs the binary as alz-rs for local use. Reviewed once as a unit by the reviewer and security-reviewer; all findings fixed or recorded. Discovered: TASK-59 to TASK-63. Next: R1 (TASK-55.2).
<!-- SECTION:FINAL_SUMMARY:END -->
