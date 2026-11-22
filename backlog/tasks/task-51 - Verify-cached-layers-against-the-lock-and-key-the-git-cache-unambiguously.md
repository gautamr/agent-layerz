---
id: TASK-51
title: Verify cached layers against the lock and key the git cache unambiguously
status: To Do
assignee: []
created_date: '2026-11-22T01:20:00Z'
updated_date: '2026-11-22T01:36:00Z'
labels:
  - area:security
  - area:resolve
dependencies: []
priority: high
ordinal: 23390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found in the TASK-50.1 security review. contentHash is computed (src/resolve/stack.ts) but never compared with the lock when a cache entry is read. Git cache keys flatten the URL (src/resolve/sources.ts), so github:acme-corp/layer and git:https://github.com-acme-corp/layer share one key. With a lock SHA set and the entry cached, no clone runs, so a crafted repository can read another layer's cached copy if it knows that layer's commit, and a private layer can end up in its build output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every cache read compares the layer's content hash with layer.lock and fails the build on a mismatch, naming the layer
- [ ] #2 Git cache keys are an unambiguous encoding of the source URL (for example a hash), and two different URLs never share an entry
- [ ] #3 Acceptance tests cover a colliding URL pair and a tampered cache entry
- [ ] #4 Account resolution (LAYERING-SPEC §7.13) checks each layer it reads against the lock's content hash, and a mismatch counts as a cache miss, which fails closed as §7.13 describes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (@architect): criterion 4 added from the TASK-50.1 final review; TASK-50.2 relies on it (its criterion 5).
<!-- SECTION:NOTES:END -->
