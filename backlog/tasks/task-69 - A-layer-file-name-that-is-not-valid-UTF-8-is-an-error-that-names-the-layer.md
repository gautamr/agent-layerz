---
id: TASK-69
title: A layer file name that is not valid UTF-8 is an error that names the layer
status: To Do
assignee: []
created_date: '2026-11-23T04:40:00Z'
updated_date: '2026-11-23T11:58:00Z'
labels:
  - discovered
  - area:rust
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
  - src/core/files.ts
priority: medium
type: bug
ordinal: 86390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found in TASK-65. Spec §5.3 'File names' says every name the walk of a layer lists, and every path a pinned layer's git tree lists, must be valid Unicode; otherwise the build stops with error [layer] naming the layer by its source (never the raw name), and account resolution counts the layer as a cache miss (layer-invalid). Rust can test this exactly with OsStr::to_str. The TypeScript reference reads names with readdirSync's default utf8 encoding, which puts U+FFFD in place of each invalid sequence, so it then fails to open the file (not run on Linux; macOS APFS refuses such names with EILSEQ, observed). decision-30: fixed only in Rust, in R3 (TASK-55.4); the acceptance case is marked reference drift. Out of scope: names that are valid UTF-8 but hold control characters.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz lint in a repository whose stack holds a layer with a file or folder named with the byte 0xFF exits 1 with error [layer] naming the layer's source, and stderr holds no byte 0xFF and no U+FFFD (acceptance lint twin, run where the file system accepts the name, skipped elsewhere, marked referenceDrift, R3.11)
- [x] #2 The resolution verdict that account resolution uses (scan_verdict with Purpose::Resolution, R3.7) counts such a layer outside the repository as a cache miss layer-invalid (Rust unit test)
- [x] #3 A pinned path: layer whose git tree lists a path that is not valid UTF-8 is refused before anything is written (Rust test)
- [x] #4 The order of checks in spec §5.3 'File names' holds: in resolution too many entries (unreadable) wins over an invalid name (layer-invalid), which wins over the content hash and .env; in the build the name error comes before the cache integrity check and env-file (Rust tests)
- [ ] #5 The alz build form of AC #1 is in the acceptance suite (written in R3.11 where it is missing) and is checked again against the Rust alz in R5 (TASK-55.6); the account-resolution form of AC #2 is checked in R6 (TASK-55.7)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-65 review (2026-11-23): the spec now quotes the message ('a file or folder name in layer <source> is not valid UTF-8; rename it', code layer, no location) and the order of checks. Reference: resolution gives unreadable (ENOENT through readRegularFileWithId, src/resolve/reader-load.ts:57 and the catch at 74). For a pinned tree, src/resolve/tree-cache.ts:90 decodes ls-tree -r -z with U+FFFD and reads blobs by id, so it probably writes and hashes the U+FFFD name (not run).

Architect (2026-11-23, owner answer 7 in TASK-55.4, delegated by the lead): criteria reworded to the lint twins that R3.11 adds; the build, check and account-resolution forms are checked again in R5 (TASK-55.6) or R6 (TASK-55.7), as the last criterion says. The other criteria are unchanged.

R3.11 (2026-11-23): AC2-4 checked by the R3.7 and R3.9 Rust tests (scan_verdict order, pinned_tree.rs a_tree_path_that_is_not_utf8...). AC1 and AC5 left unchecked: the lint and build twins are written (36-sources-and-cache-guards.test.ts, referenceDrift TASK-69, Linux only) but were skipped on this macOS machine, so they have not run. A new Linux-only Rust test for a fetched git: clone is in crates/alz/tests/git_sources.rs (compiled and linted here, not run on Linux).

2026-11-23: the Rust Linux cases ran green on PR #8's ubuntu job (a clone, a tree path and a walked name holding 0xFF; run of 4bc5132). AC #1's CLI lint twin in test 36 is Linux-only and the TypeScript acceptance suite has no Linux CI yet (TASK-4); it stays open until then or R5, whichever runs it first.
<!-- SECTION:NOTES:END -->
