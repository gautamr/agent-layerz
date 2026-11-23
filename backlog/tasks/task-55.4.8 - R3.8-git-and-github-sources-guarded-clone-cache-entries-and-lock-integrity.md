---
id: TASK-55.4.8
title: >-
  R3.8: git: and github: sources: guarded clone, cache entries and lock
  integrity
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T08:35:00Z'
labels:
  - area:rust
  - area:resolve
milestone: m-7
dependencies:
  - TASK-55.4.3
  - TASK-55.4.4
  - TASK-55.4.7
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 109390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Fixes TASK-68 (AC #4 and the wiring of AC #1). It uses R3.3, R3.4 and R3.7.
- alz-core `stack/origin.rs`: `Origin::of(&StackEntry)`:
  - `github:` gives `https://github.com/<o>/<r>.git`, remote;
  - `git:` is classified sans-IO, in the order of target-facts §11 "Clone argument is tried as a local path first": git's suffixes looked up on disk first, a `..` segment refused for a local path, then `file://`, then `scheme://` or `host:path` as remote.
  The local lookup answers come from alz-io, and a canonical path through the walk (R3.6). The clone argument for a local source is that canonical path.
- `lock/integrity.rs`: `locked_entry_for` and `verify`.
- alz-io `resolve/clone.rs`:
  1. When the lock pins the source and its entry exists, check it against the lock's content hash before `layer.yaml`, and stop there.
  2. Otherwise clone into a `TempEntry`, run `RevParseCommit` of the lock's sha or the ref, and `Checkout`.
  3. If an entry exists, compare the content hashes, discard the clone, and give the cache error on a difference. If none exists, publish the clone.
  `rev` is `ObjectId::short_rev()`.
  Errors:
  - a failed git step: `cannot fetch` with `FetchFailureLine`;
  - after git, a name that is not UTF-8: the §5.3 error;
  - a read or rename failure: `cannot store …`.
- Integration tests with real git, using a local bare repository and a fake ssh through GIT_SSH for "remote" URLs:
  - a lock pin that hits the cache, and one that misses;
  - a moved ref;
  - an existing entry against a fresh clone;
  - a tampered entry;
  - an entry that is a link;
  - `x:y` as a local folder refused for a remote clone;
  - a failed clone, a failed rev-parse and a failed checkout;
  - a cache root made read-only before publish, giving `error [cache]`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A git: or github: layer is cloned once into git-<key>@<sha> through a temporary entry, read from the cache when the lock pins it, and recorded with rev equal to the 12-character sha prefix (integration tests with real git)
- [ ] #2 A cached entry that differs from the lock's content hash, or from a fresh clone when the lock has no entry, stops with the cache error naming the layer and its directory, and a link at the entry name is refused (integration tests)
- [ ] #3 A failed clone, rev-parse or checkout gives error [source] cannot fetch with the stripped first line, and a failure to read or publish the clone gives error [cache] cannot store (integration tests; TASK-68 AC #4)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R3.3 (2026-11-23, lead): git runs in its own session, so SIGINT to alz does not reach it and alz exiting does not stop a clone. Make the runner kill the call's process group when the call is abandoned (alz interrupted, or a guard dropped), with a test that interrupts alz during a slow clone and finds no git or ssh left.
<!-- SECTION:NOTES:END -->
