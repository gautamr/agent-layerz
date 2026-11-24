---
id: TASK-55.6.12
title: 'R5.12: alz init'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T02:21:00Z'
labels:
  - area:rust
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.6.7
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 144390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10 init, §4.4).
- scaffold.rs (pure): the repo manifest, the repo layer's layer.yaml and instructions file, the layer scaffold, the mise tasks text, byte-identical to the reference; stack options parsed with the R2 source types (a bare path means path:, path:<dir>@<ref> pins a ref).
- --hub: the hub's layer/ pinned to its newest layer-v* tag through GitRunner (decision-24); the tag is parsed as a GitRef before it is written.
- Writes are create-only through RepoRoot; an existing layer.yaml is the error init; a name that is not a layer name is the error init.
- The commit identity set-up after scaffolding is R6 (TASK-55.7): R5 prints nothing for it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 alz init --kind repo and --kind layer write the reference's bytes and the created lines (assert_cmd tests and test 21 against ALZ_BIN)
- [x] #2 alz init --hub pins the hub layer to its newest layer-v* tag, and a tag that is not a valid ref is refused before anything is written (test 16's init cases and a unit test)
- [x] #3 alz init never overwrites an existing file (alz-io test)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (branch feat/rust-r5.12-init). Pure scaffold in alz-core scaffold/ (files.rs, hub.rs, stack_option.rs, layer_name.rs, error.rs); effects in alz-io: RepoRoot::{at, look_at, create_new} (create-only: temp file linked to its name, so a file, folder or link at the path is RepoError::Exists and a link above it is RepoError::Changed) and GitRunner::layer_tags (git tag --list 'layer-v*' --sort=-v:refname, TreeRead environment); wiring in crates/alz/src/command/init.rs (Surroundings moved to command.rs).
AC1 evidence: unit tests scaffold/files/tests.rs compare every scaffolded file with bytes from the reference's alz init run once in a scratch folder (esbuild bundle of src/cli, never changed); crates/alz/tests/init.rs asserts the files, modes (0644) and the created/next lines for --kind repo and --kind layer; acceptance 21-init-stack case 2 (stack, pinned path, github sources) passes against the release binary, and case 1's init assertions pass up to its build step (build is another R5 command).
AC2 evidence: unit tests hub.rs (tag not a git ref -> HubTagNotARef) and call/tests.rs (layer_tags args, read); init.rs tests: newest tag in version order (0.10.0 over 0.9.0), no tag -> no pin, 'layer-v1@2' -> 'error [init] the newest layer-v* tag of the hub is not a valid git ref', exit 1, nothing written. test 16 has no alz init case; the hub init case is in test 21.
AC3 evidence: alz-io tests/repo_create.rs (existing file kept, link at path refused with target untouched, link in a folder above refused, folder at path refused, mode 0644, no temp file left) plus alz tests/init.rs (existing layer.yaml, .layer/layer.yaml and mise.toml kept; .layer link not followed; link in .layer/instructions refused).
Checks: mise run rust:ci green (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit); mise run lint green. Acceptance with ALZ_BIN=release build: 21 passes 1 of 2 (the other waits on build); 26/28/29/36 init cases wait on R6 identity (TASK-55.7) or build. Open: --stack value errors and control characters are unspecified in the spec, so a sentence was added to 10.3; no Code::Init variant was added (alz-core diagnostic.rs untouched to avoid a clash with R5.5), the line is printed as 'error [init] ...' by the command; a hub or stack text with YAML-significant characters (' #', ': ') is written as given, as the reference does.

Verified by the lead (2026-11-24): merged into the R5 branch (465084c); mise run rust:ci and mise run lint exit 0 on the merged tree. Accepted: the §10.3 sentence refusing an invalid --stack value (reference drift). Init acceptance cases that also run alz build pass once R5.8 lands; identity-setting cases are R6.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz init writes the scaffold create-only through RepoRoot (an existing file, folder or link is never replaced; a link above a path is never written through), byte for byte as the reference's text, pinning --hub to its newest valid layer-v* tag through GitRunner.
<!-- SECTION:FINAL_SUMMARY:END -->
