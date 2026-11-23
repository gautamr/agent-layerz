---
id: TASK-69
title: A layer file name that is not valid UTF-8 is an error that names the layer
status: To Do
assignee: []
created_date: '2026-11-23T04:40:00Z'
updated_date: '2026-11-23T04:52:00Z'
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
- [ ] #1 alz build on a layer holding a file or folder named with the byte 0xFF exits 1 with error [layer] naming the layer's source, and stderr holds no byte 0xFF and no U+FFFD (acceptance test, run where the file system accepts the name, skipped elsewhere, marked referenceDrift)
- [ ] #2 Account resolution of such a layer outside the repository is a cache miss layer-invalid (acceptance test 30 or a Rust unit test)
- [ ] #3 A pinned path: layer whose git tree lists a path that is not valid UTF-8 is refused before anything is written (Rust test)
- [ ] #4 The order of checks in spec §5.3 'File names' holds: in resolution too many entries (unreadable) wins over an invalid name (layer-invalid), which wins over the content hash and .env; in the build the name error comes before the cache integrity check and env-file (Rust tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-65 review (2026-11-23): the spec now quotes the message ('a file or folder name in layer <source> is not valid UTF-8; rename it', code layer, no location) and the order of checks. Reference: resolution gives unreadable (ENOENT through readRegularFileWithId, src/resolve/reader-load.ts:57 and the catch at 74). For a pinned tree, src/resolve/tree-cache.ts:90 decodes ls-tree -r -z with U+FFFD and reads blobs by id, so it probably writes and hashes the U+FFFD name (not run).
<!-- SECTION:NOTES:END -->
