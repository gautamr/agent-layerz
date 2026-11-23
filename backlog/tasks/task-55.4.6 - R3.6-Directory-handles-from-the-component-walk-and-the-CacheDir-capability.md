---
id: TASK-55.4.6
title: 'R3.6: Directory handles from the component walk, and the CacheDir capability'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T07:59:00Z'
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
- [ ] #1 A path: directory that is HOME or above, lies inside an account folder (also through a planted link) or holds a CLAUDE_CONFIG_DIR or CODEX_HOME folder is refused before any file in it is opened, with an error naming the layer (Rust test with a fake HOME and mode-000 account folders; TASK-52 AC #1)
- [ ] #2 A cache root inside an account folder is refused the same way (Rust test; TASK-52 AC #2)
- [ ] #3 A cache entry or temporary entry that is a link is never followed, publish never replaces an existing entry, and the index entry is written through an exclusive temporary file and a rename (Rust tests)
- [ ] #4 An absolute XDG_CACHE_HOME inside the repository is refused with error [cache], a link in the cache root's own path is followed with each prefix checked, and missing folders are created with mode 0700 (Rust tests)
<!-- AC:END -->
