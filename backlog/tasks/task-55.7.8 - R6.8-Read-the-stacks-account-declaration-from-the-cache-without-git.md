---
id: TASK-55.7.8
title: 'R6.8: Read the stack''s account declaration from the cache without git'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T11:25:00Z'
labels:
  - area:rust
  - area:accounts
  - area:security
milestone: m-7
dependencies:
  - TASK-55.7.7
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 167390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.13 "No fetch on resolve", §5.3, decision-18; TASK-69).
- account/cached_reader.rs in alz-io: CachedDeclarationReader takes a read-only cache view and the repository's ReadOnlyDir, and no GitRunner, RepoRoot or StateDir, so it cannot fetch, run git or write.
- Per layer: git: and github: read the cache entry of the commit the lock pins (sha of 40 or 64 lowercase hex, exactly one entry); a pinned path: layer reads the pathref@ index entry (a regular file whose whole content is a hex id as long as the commit's), and the lock's rev must equal the manifest's ref; an unpinned path: layer outside is read in place; a layer inside the repository gives only its layer.yaml, to warn.
- The content hash is checked with scan_verdict(Purpose::Resolution) before layer.yaml is parsed; each file is read once, relative to its folder, with fstat.
- Misses: the cache location moved while mise or direnv is active, the cache inside the repository, an account folder, `..` in a git source, and the rest of §7.13.
- alz-core account/cached.rs: CachedDeclaration = None | Declared { map, layer, position, source, ref, revision } | Conflict | Miss(MissReason), with who may declare from merge/declarers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each miss reason of spec §7.13 has a Rust test that produces it, and TASK-69 #5's account form holds: a layer with a file name that is not valid UTF-8 is the miss layer-invalid (cargo test)
- [x] #2 A layer of more than 2000 entries or 16 MiB in all is unreadable, and the order of checks of §5.3 holds in resolution (cargo test)
- [x] #3 Reading a declaration leaves the cache, the repository and the state folder byte-identical and starts no process (integration test with a snapshot and a git stand-in)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (not yet verified by the full check set when this note was written): alz-core account/cached/{miss,plan,decision}.rs (MissReason with the 19 codes and phrases of §7.13/§10.3, lock_entry, read_plan, IndexEntry::tree_for, has_parent_segment, decide -> CachedDeclaration, reusing merge::refusal_reasons which was extracted from the declarers rule); alz-io fs/cache_view.rs (CacheView/CacheTrust: a read-only cache root, never created) and account/cached_reader.rs (CachedDeclarationReader::new(&ReadOnlyDir, &AbsolutePath, &AccountFolders, &CacheView), no GitRunner, RepoRoot or StateDir). Tests: crates/alz-core/src/account/cached/tests.rs (15) and crates/alz-io/tests/cached_declaration.rs (23, temporary HOME and cache, no env read or set).

Checks (run, all exit 0): mise run rust:ci (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit); mise run lint. Not run: ALZ_BIN acceptance (the reader is not wired to a command; account which is R6.10).
AC1 evidence (cargo test, crates/alz-io/tests/cached_declaration.rs): account-folder (cache in .claude-*, path: layer in .claude-*, repository in an account folder), lock-missing, lock-unreadable (link, entry without contentHash), source-invalid (.. in git URL), lock-entry-missing, lock-entry-duplicate, lock-sha-invalid, lock-ref-mismatch, cache-in-repo, layer-directory-missing, index-missing, index-invalid (malformed, wrong length, empty, link), cache-entry-missing, cache-entry-not-directory (link), content-hash (edit, file the hash does not cover, link in place of layer.yaml, layer.yaml parsed only after the check), layer-invalid (broken layer.yaml with matching hash, .env file never opened; non-UTF-8 name test returns early where the file system refuses the name, e.g. APFS, so it runs on Linux only), unreadable (2001 entries, 2x9 MiB), cache-location (XDG moved while mise active). stack-invalid and the codes/phrases are in alz-core account/cached/tests.rs. The non-UTF-8 name is layer-invalid by LayerRefusal -> MissReason (alz-core test) with the walk of alz-io/tests/layer_scan.rs.
AC2: scan_verdict(Purpose::Resolution) is the one order of checks (2000 entries, names, 16 MiB, then hash, .env, layer.yaml); the reader applies it in that order. Tests: 2001 entries and 18 MiB are unreadable.
AC3: reading_leaves_the_cache_the_repository_and_the_layers_byte_identical (read_tree before and after, three scenarios) and reading_starts_no_process (child test run with only a git stand-in on PATH; the stand-in logs nothing). The reader has no GitRunner, RepoRoot or StateDir in its signature; CacheView has no write method; a missing cache root is not created (test).
Open: acceptance with ALZ_BIN waits for R6.10; spec needs listed in the report.

Verified by the lead (2026-11-24): mise run rust:ci exits 0, src/ unchanged. The account which wiring and tests 29-31 are R6.10.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
CachedDeclarationReader reads client-layer account declarations from the cache only (no GitRunner, RepoRoot or StateDir): the lock entry, the component walk, the resolution scan, the content hash, then layer.yaml, each miss with the spec's reason; nothing is written and no process starts.
<!-- SECTION:FINAL_SUMMARY:END -->
