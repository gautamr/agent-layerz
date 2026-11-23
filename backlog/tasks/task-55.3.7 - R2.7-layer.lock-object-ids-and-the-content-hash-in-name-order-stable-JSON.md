---
id: TASK-55.3.7
title: 'R2.7: layer.lock, object ids and the content hash in name order, stable JSON'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
updated_date: '2026-11-23T06:36:00Z'
labels:
  - area:rust
  - area:cache
milestone: m-7
dependencies:
  - TASK-55.3.6
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 96390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Spec §5.3 (content hash, byte layout, .env files, file names) and §6 "Name order" (TASK-64), with decision-30's drift. Modules `crates/alz-core/src/lock.rs` with `lock/{object_id,content_hash,lock_file}.rs`, and `render.rs` with `render/json.rs`. Adds sha2 (decision-34) and serde_json (decision-33).

Types:
- `ObjectId = Sha1([u8; 20]) | Sha256([u8; 32])`, parsed only from 40 or 64 lowercase hex characters; anything else makes the lock invalid before a cache path is built (§5.3).
- `FileDigest([u8; 32])`; `ContentHash`, parsed and printed as `sha256:` plus 64 lowercase hex; `LayerFiles(BTreeMap<SafeRelPath, FileDigest>)`, so name order holds by construction; `ContentHash::of(&LayerFiles)` computes §5.3's byte layout. Hashing is pure, so it lives here; R3's walk only supplies the bytes.
- `is_env_file(&SafeRelPath)` and the skipped names (`.git`, `.DS_Store`, `node_modules`) as pure predicates for R3's walk.
- `Lock { version: V1, visibility, layers: Vec<LockedLayer { name, source, rev, sha: Option<ObjectId>, content_hash }>, files: BTreeMap<SafeRelPath, FileHash> }`.

Reading: serde_json into an alz JSON tree through alz's own visitor (a duplicate key is an error), at most 1 MiB, strict UTF-8, nesting depth at most 128 (decision-33), then conversion. Reading the file is R3's and R5's.
Writing: `render/json.rs`, an alz-owned stable JSON emitter (keys in code point order, two-space indent, the reference's escapes, a final line feed), byte for byte like the reference's `stableJson` except the TASK-64 drift.

Tests: hashes of fixtures against vectors from `test/spec/content-hash.ts` (R1.2), including two names that differ above U+FFFF and in U+E000-U+FFFF (code point order, drift); this repository's `layer.lock` parsed and emitted again byte for byte; a `sha` of 39 or 41 characters or upper-case hex refused. Property tests: emit then parse gives the same Lock; ObjectId and ContentHash round-trip through text; the emitter's key order is code point order.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ContentHash::of matches the R1.2 test-side formula on the vectors recorded in the notes, the code point order case included
- [x] #2 This repository's layer.lock parses and is emitted again byte for byte; an invalid sha or contentHash and a duplicate key are refused before any value is used (unit tests)
- [x] #3 Round-trip property tests pass for Lock, ObjectId and ContentHash
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation notes (implementer, R2.7). Added: sha2 =0.11.1 (default-features off) and serde_json =1.0.154 (default-features off, features std) to [workspace.dependencies], used by alz-core only; deny.toml [[bans.features]] for serde_json (arbitrary_precision, float_roundtrip, preserve_order, raw_value, unbounded_depth) and sha2 (alloc, oid, zeroize); alz-core clippy.toml disallows serde_json::from_*/to_*/Deserializer::from_slice/from_reader so json::parse is the only reader. New modules: json (tree, error, load), lock (hex, object_id, content_hash, lock_file), render/json (stable emitter). Tool: crates/alz-core/tools/json-emit-samples.mjs -> tests/fixtures/json-emit/samples.json (33 golden samples from the reference stableJson, biome formatted; rerun gives same bytes).
AC1 evidence: lock::content_hash::tests::content_hash_matches_the_test_side_formula_on_the_vectors. Vectors computed by bundling test/spec/content-hash.ts and running contentHash on real dirs: empty=e3b0c442...b855; one file a.md='hello\n' -> 27bef2562abfb2075cb78619ffbb0f4e21948f13622d28f9400549e5b875d4b1; nested (b.md,a/c/d,9,a/b.md,10,a-b.md) -> c06b09ad7046b7d33857803f2c3876c5624747c335003d9d4fda907f7730393b; .env/x + a.md -> 731b8e6607d2568acdbc7e191a52c728ac0e8a6ed432395b6e44da25eb53f00a; code point case (U+1F600, U+FF5E, U+E9, z, U+E000) -> c87a5f036a5a137c52c521c50c0ee1bd22d96c3ed34b77667842cf7b6e789ba9; skipped-names case (.env.dev, .envrc, .DS_Store, node_modules, sub/.DS_Store, sub/node_modules/q all excluded) -> 83bee2f69f8a476042a6a0f0d5114c3abbc1872bd99c714d9b4718c88d57deba. Also sha256 standard vectors (empty, abc) and the byte layout path NUL hex LF checked by hand.
AC2 evidence: lock::lock_file::tests::this_repositorys_lock_parses_and_is_emitted_again_byte_for_byte (include_bytes of the live layer.lock); a_lock_with_every_source_form_is_emitted_again_as_it_was_read; a_sha_that_is_not_an_object_id_is_refused... (39, 41, upper-case, empty, path-like, non-text); a_content_hash_that_is_not_sha256_and_64_lowercase_hex_is_refused; a_duplicate_key_in_any_object_makes_the_lock_invalid (top level, layer, files; diagnostic 'error [lock] layer.lock:1:N: duplicate key ...'); lockVersion, file path and shape refusals. json::tests cover 1 MiB cap (exact and +1), strict UTF-8 with position, depth, lone surrogate escape, duplicate key at any depth.
AC3 evidence: proptests a_lock_that_is_emitted_reads_back_as_the_same_lock (Lock), an_id_prints_back_as_it_was_read (ObjectId), a_content_hash_prints_back_as_it_was_read (ContentHash, FileHash), plus emitter laws (emit then parse equals the value in name order; keys in code point order; deterministic) and the order-independence of the content hash.
Emitter: byte for byte against 33 reference samples; TASK-64 drift tested by hand (U+1F600 vs U+FF5E sorts by code point; keys 10, 9, -x, a stay plain strings in name order). Floats are written as JavaScript's Number toString (tested on 18 cases).
Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests incl. 224 alz-core lib tests, doc, MSRV, deny, audit); mise run lint exit 0.
Open for the architect: (1) Depth. serde_json's limit of 128 counts the root value as level 1, so 127 nested arrays/objects are read and the 128th is refused; json::MAX_DEPTH is 127 and the test pins it. Spec §5.3 says 'nested at most 128 levels deep' - either say '127 containers' or count the scalar leaf as a level; I did not edit the spec. (2) serde_json without float_roundtrip (decision-33 keeps it off) reads a float literal with up to 1 ulp error; the reference's JSON.parse is exact. The lock holds no floats, so it does not affect R2.7, but R4/R5 reading target JSON should know. (3) Reading a lock validates source as Source and rev as GitRef, so a lock the reference accepted with a differently shaped source or rev text would be refused; check against R3 when the resolver writes rev. (4) lock fields are public (no invariant between them); unknown lock keys are ignored and dropped on write, as the reference does.

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (253 tests), mise run lint passes, src/ and test/ unchanged. Lead decisions: (1) spec §5.3 now says the JSON depth counts the top-level value as level 1, so at most 127 arrays or objects nest (what serde_json's 128 limit does, pinned by a test). (2) serde_json without float_roundtrip can read a float 1 ulp off; the lock has none; noted for R4 and R5. (3) Strict reading of source as Source and rev as GitRef stays; R3 confirms it against what the resolver writes as rev.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
layer.lock parses through the one JSON reader (serde_json, parse-only, 1 MiB cap, depth limit, duplicate keys refused) into Lock and LockedLayer with ObjectId (SHA-1 or SHA-256 hex) and ContentHash computed in name order with the §5.3 byte layout (matches the test-side formula on six vectors). alz's own stable JSON writer sorts keys by code point and gives this repository's layer.lock back byte for byte. Adds sha2 and serde_json (decisions 33 and 34).
<!-- SECTION:FINAL_SUMMARY:END -->
