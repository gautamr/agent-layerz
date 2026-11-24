---
id: TASK-55.7.4
title: 'R6.4: Read and set the repository''s git identity through alz-io'
status: To Do
assignee: []
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T08:19:00Z'
labels:
  - area:rust
  - area:identity
  - area:security
milestone: m-7
dependencies:
  - TASK-55.7.3
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 163390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§9.4, decision-24; owner questions 1 and 2 of TASK-55.7).
- git/repository.rs: RepositoryGit runs only the §9.4 calls with argument arrays and no shell: `rev-parse --git-dir` (LC_ALL=C), `rev-parse --show-toplevel`, `var GIT_AUTHOR_IDENT` and `GIT_COMMITTER_IDENT`, `config --show-scope --show-origin --get --end-of-options <key>`, `config --file <root>/.git/config --get` and `config --file <root>/.git/config --end-of-options <key> <value>`. Keys are a closed enum, values only IdentityName and EmailAddress.
- The child environment is an alz-core function next to child_environment: inherited, minus GIT_DIR, GIT_WORK_TREE, GIT_COMMON_DIR, GIT_CONFIG and GIT_INDEX_FILE, with GIT_TERMINAL_PROMPT=0.
- RepoRoot::git_config_place() -> GitConfigPlace = Writable | NotTopLevel | GitNotADirectory | ConfigNotRegular | ConfigOutside, from no-follow probes: the root is the top level, .git a real directory, .git/config absent or a regular file whose real path is inside. git follows a link at the file it writes (target-facts §11), so the probe runs right before the write and again after it; a changed file is reported.
- After a write the effective identity is read again.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With a real git in temporary repositories, a gitfile worktree, a linked .git, a linked or FIFO .git/config and a root below the top level each write nothing and give the place that says why; a file outside the repository is unchanged (integration tests)
- [ ] #2 A write adds only missing keys to <root>/.git/config, never through --global or --system, and an [include] that overrides it is reported after the re-read (integration test)
- [ ] #3 GIT_DIR, GIT_WORK_TREE, GIT_COMMON_DIR, GIT_CONFIG and GIT_INDEX_FILE pointing at another repository change nothing, while GIT_AUTHOR_*, GIT_CONFIG_COUNT and GIT_CONFIG_GLOBAL are honoured (integration test)
<!-- AC:END -->
