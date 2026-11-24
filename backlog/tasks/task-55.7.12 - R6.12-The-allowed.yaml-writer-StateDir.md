---
id: TASK-55.7.12
title: 'R6.12: The allowed.yaml writer (StateDir)'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T08:16:00Z'
updated_date: '2026-11-24T10:46:00Z'
labels:
  - area:rust
  - area:accounts
  - area:security
milestone: m-7
dependencies:
  - TASK-55.7.6
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 171390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.13 "Both files", §9.1, decision-20; AGENTS.md invariant).
- alz-io fs/state_dir.rs: StateDir, the only writer outside the repository besides the cache. Every path comes from Home, XDG_STATE_HOME and fixed names, never from repository text.
- Refuses, changing nothing, every state the reader ignores, a dangling link, anything not a regular file, and a file with an ignored entry (positions only); never repairs.
- Missing folders are made with mode 0700 only after the place is known safe; an existing folder keeps its mode and passes the checks.
- Lock file allowed.yaml.lock (named after the real file for a link): O_CREAT|O_EXCL|O_NOFOLLOW, 0600; polled every 50 ms for up to 10 s, then a refusal naming it; never removes a lock it did not create.
- Under the lock: read again; entries sorted by root; the entry for the same real root replaced; valid entries whose root does not resolve kept; `[]` after the last remove.
- Temporary file .allowed.yaml.<pid>-<8 hex>.tmp (O_EXCL|O_NOFOLLOW, 0600 by fchmod whatever the umask, flushed) renamed over the real path; a link is followed only to a file that is empty, holds [] or starts with alz's header, and the target is checked again (device and inode) just before the rename. A result over 1 MiB, or one that does not read back as written, is an error. The 8 hex come from std's RandomState, so no new crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Twelve writers at once keep every entry, and each either writes or refuses naming the lock file (integration test)
- [ ] #2 Every refused state leaves allowed.yaml, its folder and the lock file byte-identical, and nothing is ever written inside the repository, an account folder or a git work tree (integration tests and a proptest over generated places)
- [ ] #3 For ordinary roots the bytes written equal the reference's allowed.yaml for the same entries (parity test against recorded samples)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (alz-io account/state_dir.rs, state_place.rs, state_error.rs; StateDir, AllowChange, Updated, Stage, StateError, StateProblem exported). Layout note: it sits beside AccountFiles in account/, not fs/, because it reuses the reader's private trust helpers; the checks are the reader's own (AccountFiles::allow_state under the lock). AC1: tests/state_dir.rs twelve_writers_at_once... (12 threads, every Ok entry present, each failure is LockHeld naming allowed.yaml.lock) and a_second_writer_waits... (deterministic, via Stage observer). AC2: refusal tests snapshot the watched tree (kind, mode, bytes, link target) before and after: inside repository, account folder, git work tree (missing and existing), moved XDG with mise, bad HOME, no user id, wrong owner, group-writable file or folder, dangling link, not a regular file, not a list, ignored entries by position, link to a foreign file, lock held or a link; proptest nothing_is_ever_written_inside_the_repository_an_account_folder_or_a_work_tree over generated places. Crash: a child process of the test binary aborts at Stage::TempWritten; the old file is whole, the 0600 temp file and the lock stay and the next run names the lock. AC3: the_written_bytes_equal_the_reference_samples_and_the_core_emitter (recorded text of test/reference/32-account-allow.test.ts and AllowState::to_yaml). Modes 0700 for created folders (existing keep theirs), 0600 for file, lock and temp. Decisions to confirm: XDG_STATE_HOME empty or relative counts as unset (spec rule), not a refusal; the identity check before the rename is made for every existing file, not only a link (stricter); a Set larger than 1 MiB by itself is refused before any folder is made. ResolvedAllowState gained is_list(). Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit); mise run lint ok.

Lead (2026-11-24): accepted — empty or relative XDG_STATE_HOME counts as unset (spec); the inode check before rename for every existing file (stricter); a Set over 1 MiB refused before any folder is made; StateDir beside AccountFiles to reuse the trust checks.
<!-- SECTION:NOTES:END -->
