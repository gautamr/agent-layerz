---
id: TASK-50.9
title: Read a stack's account declaration from the layer cache without git
status: To Do
assignee: []
created_date: '2026-11-22T03:33:00Z'
updated_date: '2026-11-22T03:38:00Z'
labels:
  - area:accounts
  - area:resolve
  - area:security
milestone: m-3
dependencies:
  - TASK-50.8
references:
  - docs/LAYERING-SPEC.md
  - src/resolve/cache-integrity.ts
parent_task_id: TASK-50
priority: high
type: feature
ordinal: 26390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Second slice of TASK-50.2. Account resolution runs at every prompt after a cd (shell hook) and before every alz run, so it must never fetch, clone or run git, and must write nothing (LAYERING-SPEC §7.13 'No fetch on resolve'). This slice gives resolution a reader that finds each layer of an allowed repository's stack in the layer cache, or in place for an unpinned path: layer, checks it against layer.lock's content hash with TASK-51's verifyCachedLayer, and applies the first slice's who-may-declare rule. alz build gains the pathref@ cache index for pinned path: layers.

Anything the reader cannot verify is a cache miss (TASK-50.2 implementer note from the TASK-51 review): no lock entry, a different sha, a missing or malformed field, an unreadable entry, or a layer.yaml that is not a regular file. TASK-51's lockedEntryFor returns nothing for an unpinned path: layer (no sha), so the reader needs its own entry selection there.

No CLI command yet; the next slice calls the reader. Cache-only change: generated output and layer.lock do not change.

Spec: LAYERING-SPEC §5.3, §6, §7.13 'No fetch on resolve', §9.1 (the index lives in the cache), §13 test 20 (index cases).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A pinned path: layer is found through a cache index entry that alz build writes next to each tree2@<tree> entry, keyed by the layer directory's real path and the commit SHA (for example pathref@<sha256 of the real path>-<sha>) and naming its tree; build and resolution compute the real path with the same function, resolution computes it from the manifest without git, uses the lock sha only when it is 40 or 64 hex characters, reads only the index entry for that path, and uses it only when lstat shows a regular file whose whole content is 40 or 64 hex characters; a path: layer without a ref is read in place (only its layer.yaml is used, with the LAYERING-SPEC §7.12 inside/outside test); anything else is a cache miss, and layer.lock does not change (tests: index hit, a malformed sha, another path's index entry, a malformed index entry, a symlinked index entry)
- [ ] #2 The reader never fetches, clones or runs git and writes nothing; it reads only the layers it can locate as in criterion #1, with build's validation (layer.yaml a regular file, parsed only after the content check), checks each against layer.lock's content hash (TASK-51), and counts a mismatch, a missing lock entry or field, a lock sha that does not belong to the entry it read, or a cache entry that is not a real directory as a cache miss (tests trace git calls and file reads)
- [ ] #3 For a stack it can read, the reader yields the winning account declaration (the per-target map, the declaring layer, its revision, and its source in the form allowed.yaml records: a remote URL without its ref, or a local layer's real path) and each declaration it ignores because its layer may not declare, by the same rule the build applies (tests: the client layer's map wins, a ref bump leaves the recorded source unchanged, a repo-layer declaration is reported as ignored)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 owner: layers inside the repository are read only to warn when one declares an account; resolution does not check them against the lock, so editing .layer/ never blocks the account.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-22T03:35:00Z

---

Owner question (affects this task's reader and TASK-50.10; blocks the plan of this task only). LAYERING-SPEC §7.13 'No fetch on resolve' checks every layer resolution reads against the lock's content hash. That includes the repository's own unpinned `path:./.layer`, so in an allowed repository any edit under `.layer/` since the last `alz build` is a cache miss: alz run refuses, and the hook keeps the recorded account until `alz build`. A layer inside the repository can never declare, so its content cannot change the resolved account. Options: (a) as written: hash every layer; (b) layers inside the repository are read only to warn about an ignored declaration (layer.yaml only, size-limited) and never count as a miss; (c) do not read layers inside the repository at all, which drops the 'ignored with one warning' case for the repo layer from §13 test 24. Recommendation: (b). Layers outside the repository are hashed either way.

---
<!-- COMMENTS:END -->
