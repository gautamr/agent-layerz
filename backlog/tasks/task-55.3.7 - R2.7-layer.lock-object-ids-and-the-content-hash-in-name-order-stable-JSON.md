---
id: TASK-55.3.7
title: 'R2.7: layer.lock, object ids and the content hash in name order, stable JSON'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
updated_date: '2026-11-23T05:27:00Z'
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
- [ ] #1 ContentHash::of matches the R1.2 test-side formula on the vectors recorded in the notes, the code point order case included
- [ ] #2 This repository's layer.lock parses and is emitted again byte for byte; an invalid sha or contentHash and a duplicate key are refused before any value is used (unit tests)
- [ ] #3 Round-trip property tests pass for Lock, ObjectId and ContentHash
<!-- AC:END -->
