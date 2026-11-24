---
id: TASK-55.7.8
title: 'R6.8: Read the stack''s account declaration from the cache without git'
status: To Do
assignee: []
created_date: '2026-11-24T08:15:00Z'
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
- [ ] #1 Each miss reason of spec §7.13 has a Rust test that produces it, and TASK-69 #5's account form holds: a layer with a file name that is not valid UTF-8 is the miss layer-invalid (cargo test)
- [ ] #2 A layer of more than 2000 entries or 16 MiB in all is unreadable, and the order of checks of §5.3 holds in resolution (cargo test)
- [ ] #3 Reading a declaration leaves the cache, the repository and the state folder byte-identical and starts no process (integration test with a snapshot and a git stand-in)
<!-- AC:END -->
