---
id: TASK-55.7.6
title: 'R6.6: The home folder and the two account files in alz-io'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T08:20:00Z'
labels:
  - area:rust
  - area:accounts
  - area:security
milestone: m-7
dependencies: []
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 165390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.13 "User config", "Allow state", "Both files", "Environment"; TASK-72; owner question 3).
- account/home.rs: HOME as given; unset: the user database lookup of std::env::home_dir (no new crate); no entry: not an absolute path, so both files are ignored with one warning and named accounts fail closed.
- XDG_CONFIG_HOME and XDG_STATE_HOME: empty or relative counts as unset, a relative one with a warning.
- account/files.rs: AccountFile<Located> -> Trusted | Ignored(reason): inside the repository, not owned by the user, writable by group or others (file, folder, parents below HOME), the folder of a link and of its target, an XDG folder moved while mise or direnv is active (presence only), allowed.yaml inside a git work tree by lstat of .git (accounts.yaml there only warned), a path that names an account folder as written (before any file system call), no user id.
- Reads are no-follow, O_NONBLOCK, regular by fstat, at most 1 MiB plus one byte, with the alias limit, into alz-core's UserConfig and AllowState; folder keys and allowed roots are checked again once real; two that resolve alike are both ignored.
- Every map read from the two files is an ordered map keyed by validated values; a key outside its grammar is ignored with a warning, whatever the key (table rows of unusual keys, without comment).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 TASK-72 #1 holds: with HOME unset and no user database entry (an injected lookup), resolution warns once that HOME is not an absolute path and gives the default account for every target (Rust test)
- [ ] #2 Each ignore reason of 'Both files' has an alz-io integration test with a temporary HOME, and a FIFO in place of either file is reported unreadable without waiting (cargo test)
- [ ] #3 Property: an ignored file contributes no entry, and no warning holds a value or an invalid name from the file (proptest)
<!-- AC:END -->
