---
id: TASK-55.4.6
title: 'R3.6: Directory handles from the component walk, and the CacheDir capability'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T08:27:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.4.5
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 107390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Fixes TASK-52 (AC #1 and AC #2) in alz-io.
- `fs/dir.rs`: `Dir`, opened only by driving `RealPathWalk` (R3.5) with rustix:
  - each component is opened with `openat(O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC)` relative to the handle before it;
  - links are answered with `readlinkat`;
  - a refusal stops the walk before the component it names is opened.
  `ReadOnlyDir::open` stays for paths a person typed. A manifest's `path:` directory always goes through `Dir`. `Dir::read_file` keeps R2's no-follow, O_NONBLOCK, fstat S_ISREG and cap-plus-one rules.
- `fs/cache_dir.rs`: `CacheDir::open_or_create(CacheLocation)` goes through the walk, refuses an account folder, creates each missing component with `mkdirat` mode 0700, follows a link in the root's own path with each prefix checked, and refuses an absolute XDG_CACHE_HOME inside the repository with error [cache] (owner answer 6). It provides:
  - `entry(&EntryName) -> Option<Dir>`: a link or a non-directory there is the cache error, and is never followed;
  - `TempEntry`: `.tmp-<pid>-<n>`, made with an exclusive `mkdirat` 0700, its handle checked with fstat (owner and directory), and removed on drop, again without following links;
  - `publish(TempEntry, &EntryName)`: `renameat_with` no-replace. On EEXIST the existing entry is returned for checking;
  - `write_index` and `read_index`: a temporary file made with O_CREAT|O_EXCL|O_NOFOLLOW 0600, then a rename; a regular file of 40 or 64 hex characters only.
- Tests in decision-27's list:
  - links planted in a path component and in a cache entry;
  - a trailing slash;
  - a name swapped for a link between two calls;
  - a fake HOME with `~/.claude-x`, CLAUDE_CONFIG_DIR and CODEX_HOME folders made mode 000, so any access inside them fails visibly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A path: directory that is HOME or above, lies inside an account folder (also through a planted link) or holds a CLAUDE_CONFIG_DIR or CODEX_HOME folder is refused before any file in it is opened, with an error naming the layer (Rust test with a fake HOME and mode-000 account folders; TASK-52 AC #1)
- [x] #2 A cache root inside an account folder is refused the same way (Rust test; TASK-52 AC #2)
- [x] #3 A cache entry or temporary entry that is a link is never followed, publish never replaces an existing entry, and the index entry is written through an exclusive temporary file and a rename (Rust tests)
- [x] #4 An absolute XDG_CACHE_HOME inside the repository is refused with error [cache], a link in the cache root's own path is followed with each prefix checked, and missing folders are created with mode 0700 (Rust tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in an isolated worktree at 5d76274 and merged by the lead (2026-11-23); alz-io lib.rs export conflict with R3.3 resolved by hand; rustix's process feature was already added by R3.3. After the merge mise run rust:ci exits 0 (484 tests: 11 directory_handles and 17 cache_dir integration tests). New in alz-io fs/: walk.rs (drives RealPathWalk with statat/readlinkat relative to parent handles), dir.rs (Dir::open), read_below.rs (shared no-follow capped read), cache_dir.rs (CacheDir, TempEntry with publish, index read/write by O_EXCL 0600 and rename), walk_error.rs, cache_error.rs. alz-core gains lies_within and CacheBase. Lead decisions on open points: (1) the caller in R3.7/R3.8 resolves HOME, CLAUDE_CONFIG_DIR and CODEX_HOME to real paths once and passes them to with_real_home/with_real_configured, and builds RepositoryPaths — recorded on TASK-55.4.7. (2) Wording 'cannot use the layer cache at <dir>', 'layer <s> is not a directory', 'cannot be opened': kept; R3.11 adds them to the spec with R3.4's messages. (3) remove_tree recursion (one fd per level) goes to the R3 unit review; an iterative version if the review asks. (4) No race test for a swap between mkdirat and openat; the owner check on the handle is the guard.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz-io reads layer folders and the layer cache only through handles: Dir::open walks a path one component at a time (statat/readlinkat relative to the parent, never asking inside an account folder) and refuses layers in or around account folders; CacheDir opens or creates the cache root under the owner's rules (inside the repository or an account folder refused, a link in the root's own path followed, missing levels 0700) and publishes entries from owner-checked temp folders without following links; the cache index is written exclusively and renamed.
<!-- SECTION:FINAL_SUMMARY:END -->
