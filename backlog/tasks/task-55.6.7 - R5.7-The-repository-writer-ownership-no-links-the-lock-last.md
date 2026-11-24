---
id: TASK-55.6.7
title: 'R5.7: The repository writer: ownership, no links, the lock last'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:08:00Z'
updated_date: '2026-11-24T00:03:00Z'
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
- [x] #1 A path that exists, was not generated and is not owned, and any path under a link or that is not a regular file, is a collision; nothing is written through a link (alz-io integration tests with links, FIFOs and case-variant names)
- [x] #2 plan_workspace after apply gives no changes, and apply never touches a path the plan does not name (proptest over trees and disk states)
- [x] #3 An entry of the previous lock alone never makes a file deletable or overwritable beyond what R5.1's §4.3 allows (unit tests with a lock naming files alz did not write)
- [x] #4 The .gitignore block matches the reference's bytes and replaces a legacy block (unit tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (R5.7, branch feat/rust-r5.7-writer). Design: pure plan in alz-core render/write_plan (plan_writes over FileTree, Lock, previous Lock, Disk, Protected; ClearPlan only exists without collisions, so apply cannot run with one) and render/gitignore (synced); effects in alz-io fs/repo_root (RepoRoot::observe / apply, rustix only, no unsafe). Plan said workspace.rs; kept to new files render/write_plan*.rs per the coordination note. Evidence: AC1: alz-io tests/repo_root.rs (link at path with a lock entry holding the target's hash, link above a path, a folder turned into a link after the plan -> RepoError::Changed and outside untouched, FIFO/folder/FIFO-as-parent/FIFO in managed folder refused without blocking, managed folder walk never enters a link, case-variant name) and write_plan unit tests. AC2: write_plan tests property 'planning again after the plan is applied gives no changes' (pure simulation) and alz-io proptest 'applying a plan changes only the paths it names' (real files, snapshot diff of repo + outside folder, then replan settled and a second apply is a no-op). AC3: unit tests 'a lock entry with another hash does not make an unmarked file alz's own', 'a file the lock lists is deleted only when it is alz's own', 'a lock entry never deletes a protected path or the lock', property 'a change only names a path that is absent or alz's own'. AC4: render/gitignore tests (new block, separator, byte-for-byte outside text, legacy block replaced, block removed, no-end block, idempotent; an empty tree keeps the blank line the reference writes). Also: temp+rename test (failure at the second write leaves first file complete, no temp file, no lock), modes 0644/0755, lock written last and only when changed, deletes only owned files with empty folders pruned and root kept. Checks: mise run rust:ci green (fmt, clippy -D warnings, tests, doc, msrv, deny, audit); mise run lint green. Spec: new paragraph 'What the writer compares' in section 4.3 (1 MiB comparison cap, managed-folder walk, case-variant rule, reserved layer.lock/.gitignore never deleted, non-UTF-8 .gitignore, Changed error). Open: a lock listing any file with its matching hash lets a build delete it (owner rule); consider also refusing layer.yaml/.layer paths. fsync of the folder after rename not done. Pipeline wiring and message rendering (Display of CollisionReason is the spec text) belong to R5.8.

Follow-up (lead decision): alz's own inputs, layer.yaml at the root and anything under .layer/ (compared after case folding), are in the same never-touched list as layer.lock and .gitignore: never deleted whatever the lock or hash says, and a tree path there is a collision with reason Input. Tests: write_plan unit test and alz-io real-file test (lock listing layer.yaml and .layer/x.md with matching hashes: both stay); one spec sentence in section 4.3 'What the writer compares'. rust:ci and lint green. Possible later hardening: fsync of the folder after the rename (the reference does none).

Verified by the lead (2026-11-24): merged into the R5 branch (15c091f); mise run rust:ci and mise run lint exit 0 on the merged tree. Lead decision applied: layer.yaml and anything under .layer/ are never written or deleted (9213795). Folder fsync left for later hardening.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
RepoRoot in alz-io is the only writer inside the repository: a pure write plan in alz-core (owned = the generated mark, or a lock entry whose hash matches the bytes; links, non-regular files and alz's own inputs are collisions), applied with rustix through temporary files renamed into place and the lock written last; the .gitignore block is kept in sync.
<!-- SECTION:FINAL_SUMMARY:END -->
