---
id: TASK-55.4.9
title: >-
  R3.9: Pinned path: layers: tree2@ entries from blobs, verification and the
  index
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T09:19:00Z'
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
- [x] #1 A pinned path: layer is written to tree2@<tree> from its blobs, each id checked in Rust, with modes 0644 or 0755, verified before and on every later read against git ls-tree -r, and indexed by pathref@<hash>-<sha> (integration tests with real git)
- [x] #2 A tree path that is not valid UTF-8, a .git or dot segment, a backslash, or two names equal after NFC and lowercasing is refused before anything is written (unit tests on TreeListing; TASK-69 AC #3)
- [x] #3 Replace refs, attributes files and missing blobs do not change what is written: missing blobs are an error, and a link or submodule at the layer path is cannot resolve ref (integration tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (R3.9), checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit), mise run lint exit 0.

Code: alz-core git/blob_id.rs (ObjectFormat, BlobId::of; sha1 =0.11.0 default-features off per decision-36, deny.toml feature ban + wrappers=[alz-core]), git/tree.rs (TreeListing::from_entries over parsed ls-tree -r -z entries; ListingRefusal NotUtf8 > NotSafe > Collision), git/tree_check.rs (verify_entry, EntryDifference), layer/scan.rs (GitFileFacts, with_git_facts, with_other_entry), stack/resolution_failure.rs (cannot_read_pinned, cache_differs_from_tree, pinned_tree_refused). alz-io: GitCall::RevParseTree (+ GitRunner::rev_parse_tree, Outcome::Commit renamed Object) for a layer at the repository top, fs/write_below.rs + TempEntry::write_file (mkdirat 0700, O_EXCL|O_NOFOLLOW, fchmod 0644/0755), Dir::scan_against_tree (blob id and exec bit recorded as the bytes are read), resolve/pinned_tree.rs (PinnedTreeReader, PinnedLayer, PinnedRead). Spec §6 gained the refusal messages and the top-of-repository and error-kind rules.

AC1: crates/alz/tests/pinned_tree.rs with real git: written from blobs (eol/.gitattributes no effect, work tree edits and untracked files not read, modes 0644/0755, no .git, temp folder gone), ref and sha name one entry, docs commit moves commit not tree (entry still verified, own index per commit), index written/rewritten/folder at name is cache error/read-only cache is cache-index warning, five tamperings (changed, extra, missing, mode, link) refused on every read, skipped names, entry that is a link, lock hash checked only for cache read of the pinned sha, SHA-256 repository, BlobId equals git hash-object --no-filters.
AC2: unit tests in git/tree/tests.rs (NotUtf8 wins, .git/.GIT/.git./backslash/../empty, NFC+case collisions, file vs folder, proptests) plus integration tests: non-UTF-8 name, .., .GIT, .git., backslash, a vs A, NFC vs NFD all refused with nothing in the cache. The non-UTF-8 test builds the tree with git plumbing (mktree -z), so it runs on macOS too and is not cfg(target_os = linux).
AC3: replace ref ignored (GIT_NO_REPLACE_OBJECTS; original bytes written), .gitattributes/eol no effect, blob:none --no-checkout clone fails with 'the repository does not hold the file' and writes nothing (skipped below git 2.44), link or submodule at the layer path is cannot resolve ref, links/submodules inside the tree skipped, NFC-named entry.
Limits: pinned-tree resolution is a build read, so Purpose::Build (no 2000/16 MiB budgets); Dir::scan_against_tree keeps them for Purpose::Resolution (layer_scan.rs test: 2001 entries refused, same layer read in full in the build). The 64 MiB output cap for listings and batches is unchanged; a layer whose blobs total more than 64 MiB fails with 'git wrote more output than alz accepts' (not tested: a 64 MiB fixture).
Open: skipped-name files in a tree are checked by path (read and blob id), not by executable bit; an extra empty folder in an entry is not a difference (not a file). Not wired into a command yet (R3.10).

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (596 tests; pinned_tree 22), lint passes, src/ and test/ unchanged. Accepted deviations: the non-UTF-8 tree name is built with git mktree -z, so it runs on macOS too (no Linux gate); a pinned tree is a build read (no 2000-entry/16 MiB budgets), bounded by the 64 MiB git output cap. For the R3 review: skipped-name files are checked by path and blob id but not executable bit; an extra empty folder in an entry is not a difference.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Pinned path: layers are written into tree2@ cache entries straight from git's blobs (no checkout, no working tree, no attributes, replace refs off), refused before anything is written when a tree path is not valid UTF-8, unsafe or collides under NFC and case, verified against the tree on every read (changed, extra, missing, mode, link), and found again through the pathref@ index. Adds sha1 (decision-36) for SHA-1 blob ids.
<!-- SECTION:FINAL_SUMMARY:END -->
