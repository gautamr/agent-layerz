---
id: TASK-55.4.9
title: >-
  R3.9: Pinned path: layers: tree2@ entries from blobs, verification and the
  index
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T07:59:00Z'
labels:
  - area:rust
  - area:resolve
milestone: m-7
dependencies:
  - TASK-55.4.8
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 110390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Fixes TASK-69 (AC #3). It adds sha1 =0.11.0 with default features off (decision-36) for SHA-1 blob ids, and uses unicode-normalization (decision-37) for NFC.
- alz-core:
  - `git/tree.rs`: `TreeListing::parse(ls-tree -r -z bytes)`:
    - mode, type, id and path;
    - a path that is not UTF-8 is the §5.3 error, before anything is written;
    - a backslash, a `.` or `..` segment, or `.git` in any case or with a trailing dot or space is refused (SafeRelPath);
    - links (120000) and submodules (commit) are skipped;
    - two paths equal after NFC and lowercasing, or a file and a folder like that, are refused.
  - `TreeEntryCheck` compares a `LayerScan` of an entry with the listing: NFC names, the executable bit equal to the mode, no extra file, no link, and skipped names ignored unless the tree has them.
  - `git/blob_id.rs`: `BlobId::of`.
- alz-io `resolve/pinned_tree.rs`:
  1. `ShowToplevel`, `ShowPrefix`, `RevParseCommit ref`, then `LsTreeAt`, which must list the prefix as `040000 tree`; otherwise `cannot resolve ref`.
  2. `LsTreeRecursive`, then `CatFileBatchCheck`: every blob must be present, as a blob.
  3. An existing `tree2@` entry is verified against the listing, whatever the lock says.
  4. Otherwise the entry is written into a `TempEntry` from `CatFileBatch`, each blob's id checked in Rust, with modes 0644 or 0755 set explicitly, verified, and published.
  5. `pathref@` is written; a failure there is a `cache-index` warning.
  6. When the lock pins the same sha, the content hash is checked as in §5.3.
- Integration tests with real git:
  - a docs commit that moves the commit but not the tree;
  - replace refs ignored;
  - a hub cloned with blob:none that lacks blobs;
  - `.gitattributes`, eol and filters having no effect;
  - a submodule or a link at the layer path;
  - a name that is not NFC.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A pinned path: layer is written to tree2@<tree> from its blobs, each id checked in Rust, with modes 0644 or 0755, verified before and on every later read against git ls-tree -r, and indexed by pathref@<hash>-<sha> (integration tests with real git)
- [ ] #2 A tree path that is not valid UTF-8, a .git or dot segment, a backslash, or two names equal after NFC and lowercasing is refused before anything is written (unit tests on TreeListing; TASK-69 AC #3)
- [ ] #3 Replace refs, attributes files and missing blobs do not change what is written: missing blobs are an error, and a link or submodule at the layer path is cannot resolve ref (integration tests)
<!-- AC:END -->
