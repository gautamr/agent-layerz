---
id: TASK-55.6.7
title: 'R5.7: The repository writer: ownership, no links, the lock last'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T18:08:00Z'
updated_date: '2026-11-23T23:17:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.6.1
  - TASK-55.6.2
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 139390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§4.3, §4.5, decision-27).
- alz-io fs/repo_root.rs: RepoRoot, the only capability that writes inside the repository. observe() reads each tree path, each file of the managed folders and each path of the previous lock one component at a time with O_NOFOLLOW: Absent, Regular { bytes up to the cap, executable }, or NotRegular (a link, a folder, a FIFO, a device).
- alz-core workspace.rs: plan_workspace, pure. Create when absent; a path is owned only as owner question 1 decides (the rule of R5.1's §4.3, the previous lock being untrusted); an existing path that is not owned, or not a regular file, or under a link, is a collision; update when bytes or the executable bit differ; delete an owned path the build no longer makes; files in managed folders that are neither generated nor owned are collisions; the lock text and whether it changed.
- apply(): each write as a temporary file in its folder (O_EXCL, fchmod 0644 or 0755) renamed over the target with renameat; deletes; empty folders pruned up to the root; layer.lock written last; missing folders made with mkdirat and opened no-follow.
- gitignore.rs: the managed block for commit_generated: false, current and legacy markers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A path that exists, was not generated and is not owned, and any path under a link or that is not a regular file, is a collision; nothing is written through a link (alz-io integration tests with links, FIFOs and case-variant names)
- [ ] #2 plan_workspace after apply gives no changes, and apply never touches a path the plan does not name (proptest over trees and disk states)
- [ ] #3 An entry of the previous lock alone never makes a file deletable or overwritable beyond what R5.1's §4.3 allows (unit tests with a lock naming files alz did not write)
- [ ] #4 The .gitignore block matches the reference's bytes and replaces a legacy block (unit tests)
<!-- AC:END -->
