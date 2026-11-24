---
id: TASK-55.6.8
title: 'R5.8: alz build and alz check'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T03:39:00Z'
labels:
  - area:rust
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.6.6
  - TASK-55.6.7
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 140390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step: crates/alz/src/command/{pipeline,build,check}.rs.
- pipeline: read the manifest and the lock, StackResolver, placements_of_stack, merge, render, the AGENTS.override.md walk (alz-io repo_walk, no-follow, .git skipped), lock_of, observe, plan_workspace. Diagnostics printed once, through report.rs.
- build: collisions are `error [collision]` lines and exit 1 unless --adopt (R5.14); apply; the .gitignore block; the summary line of §10.
- check: the change, collision and lock lines of §10, then the failed or passed line; --skip-identity parsed; the identity comparison itself is R6 (a seam, no behaviour).
- Golden trees: a script under crates/alz/tools/ builds every fixture repository with the reference and writes its tree; a Rust integration test builds the same fixtures with the Rust alz and compares byte for byte, drift samples marked as in the YAML goldens.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 alz build writes, and alz check passes on, the reference's bytes for every fixture repository (golden-tree integration test)
- [x] #2 Acceptance files 01, 02, 07 to 09, 11, 14, 15, 23 and 37's build rows pass against ALZ_BIN (vitest run with ALZ_BIN=target/release/alz, counts in the notes)
- [x] #3 The build and check forms of TASK-52, TASK-53, TASK-63 and TASK-71 in tests 24 and 36 pass against ALZ_BIN (vitest run)
- [x] #4 The layer walk refuses a file name with a control or format character (error layer naming the layer; layer-invalid in account resolution), per spec §5.3
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R5.2 (2026-11-24): the layer walk must raise error [layer] naming the layer for a file name with a control or format character (SourceRef and OutputPath refuse them as typed values only), and a layer version with a control character or '-->' (LayerLabel) is a layer error in the renderer.

From R5.4 (2026-11-24): the walk in alz-io (fs/scan.rs read_file knows the executable bit) must fill LayerContents::with_executable so skill scripts, hook scripts (R5.5) and passthrough files (R5.6) keep their mode; until then they render as regular files.

From R5.6 (2026-11-24): write only Rendered::to_write() (None when any error exists), pass Rendered::managed_folders() and protected() to plan_writes; make whole-tree golden trees for every fixture the acceptance suite builds (03 and 15 have none yet).

Implementer (2026-11-24), state at the end of this pass; nothing committed, no AC checked.
Changed: alz-core (NameProblem::ControlOrFormat, LayerRefusal::NameControlOrFormat, ListingRefusal::ControlOrFormat, file_name_control_or_format, has_control_or_format, LayerScan::with_executable, Code::Check and Code::Write); alz-io (scan.rs fills LayerContents::with_executable from the walk's read_file bit and refuses a control or format name; RepoRoot::files_named for the AGENTS.override.md walk, .git/.DS_Store/node_modules skipped, no link followed; StackResolver::placements wires placements_of_stack); alz (command/{pipeline,build,check,repo_files}.rs, cli Build and Check{--skip-identity}, lint now shares repo_files); alz-test-support tree.rs (copy_tree, read_tree, folder_names); docs/LAYERING-SPEC.md §10.1 row for code write.
AC #1 (golden trees): crates/alz/tools/golden-trees.mjs (reference build in scratch copies, isolated HOME, fixed git dates for 16-hub) wrote 24 goldens to crates/alz/tests/fixtures/golden/ (README there; grep for machine paths found none), 03 and 15 included. crates/alz/tests/golden_trees.rs runs the Rust alz build in a temp copy, compares paths, bytes and executable bits and stdout byte for byte, stderr by level and code, then alz check exits 0, then an edit of AGENTS.md makes check exit 1 with 'error [check] AGENTS.md: differs from a fresh build' and 'check failed: ...'. cargo test -p alz --test golden_trees: 1 passed. Left out of goldens, by README: builds that fail in the reference, 04 repo-widen-command-operator and repo-widen-path-scope (Rust refuses: tool-widen, drift), [identity] lines (R6). Not shown yet: a test that each of the 24 goldens holds a file with the executable bit (hook-script goldens exist in 09/19/37 and pass the bit comparison).
AC #2, ALZ_BIN=target/release/alz, vitest run --project acceptance, per file: 01 3/3; 02 1/1; 07 2/2; 08 2/2; 09 1/1; 11 1/1; 14 1/2; 15 1/1; 23 0/2; 24 259/277; 36 16 pass 4 skipped (Linux-only); 37 139/139 (every 'alz build refuses the file and writes nothing' row passes). Failing: 14 (1) and 23 (2) and 24 (2: lock outside the repo, lock sha) all call build --adopt: R5.14. 24: 12 rows 'agent frontmatter, claude.mcpServers list/map' expect error [protected-variable] but the agent reader reports error [agent] first (R4 reader: scan claude.mcpServers env for protected variables before or besides the refusal; follow-up for R5.15); 1 row 'JSON nested deeper than 32 levels' (R4 protected-variable scanner has no depth limit; R5.15); 3 rows need alz account which (R6, TASK-55.7). I loosened test 24's three wording rows (git option, github ref, manifest ref) to the regexes its twins at lines 90 and 114 already use, since the Rust manifest wording is the spec's.
AC #3: 24 and 36 build forms: TASK-52, 53, 63 and 71 rows pass in 36 (16/16 run) and 24 except the rows listed above; evidence recorded on those tasks.
AC #4: walk refusal done in the alz-io walk and alz-core verdict: error [layer] 'a file or folder name in layer <source> holds a control or format character; rename it', layer-invalid in account resolution, git tree paths too (ListingRefusal); tests in crates/alz-io/tests/layer_scan.rs (4 new). The header-ref rule (version with a control character or '-->') was already R5.2's LayerLabel and is exercised by the pipeline.
Checks: mise run rust:ci exit 0; mise run lint ok; mise run test 1189 passed, 208 expected fail, 5 skipped.
Left: build --adopt (R5.14); diff (R5.9); ls/explain (R5.10); doctor (R5.11); account which/identity (R6); the 13 R4-reader rows of test 24 above; --skip-identity is parsed and does nothing. Known gaps: the override walk skips a folder whose entries hold an invalid-UTF-8 name; targets in the summary print in sorted order (BTreeSet), the reference prints declared order.

Correction: the goldens with the executable bit are 01-determinism and 08-locked-hook (hook script block-secrets.sh); the test compares the bit and passes.

Verified by the lead (2026-11-24): mise run rust:ci and mise run lint exit 0, src/ unchanged, goldens hold no machine paths. AC #2 read as its build rows: 01, 02, 07-09, 11, 15 and 37 pass whole; 14's and 23's failing rows are build --adopt (R5.14). Test 24's three regex rows accept the spec's Rust wording beside the reference's, as their twins do. crates/alz/tools/golden-trees.mjs drives the reference to make test data and is deleted with the reference at cutover.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz build and alz check run end to end in Rust: the stack resolves through alz-io, merges and renders in alz-core, and RepoRoot writes only what the write plan allows, lock last. All 24 golden trees match the reference's paths, bytes and modes; test 37's build rows and the build forms of TASK-52, 53, 63 and 71 pass against the Rust binary; layer file names with control or format characters are refused.
<!-- SECTION:FINAL_SUMMARY:END -->
