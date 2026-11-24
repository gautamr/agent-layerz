---
id: TASK-55.7.4
title: 'R6.4: Read and set the repository''s git identity through alz-io'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T10:23:00Z'
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
- [x] #1 With a real git in temporary repositories, a gitfile worktree, a linked .git, a linked or FIFO .git/config and a root below the top level each write nothing and give the place that says why; a file outside the repository is unchanged (integration tests)
- [x] #2 A write adds only missing keys to <root>/.git/config, never through --global or --system, and an [include] that overrides it is reported after the re-read (integration test)
- [x] #3 GIT_DIR, GIT_WORK_TREE, GIT_COMMON_DIR, GIT_CONFIG and GIT_INDEX_FILE pointing at another repository change nothing, while GIT_AUTHOR_*, GIT_CONFIG_COUNT and GIT_CONFIG_GLOBAL are honoured (integration test)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R6.4 (implementer, 2026-11-24). Files: alz-core git/environment.rs (repository_environment, RepositoryMessages; tests in environment/tests.rs), alz-io git/repository.rs (RepositoryGit, KeyWrite, Stopped, SetupResult, RepositoryCall, Answer), git/runner.rs (run_repository), fs/repo_root/git_config.rs and repo_root.rs (git_config_place, seal_git_config, verify_git_config, is_this_folder; GitConfigSeal, Changed), crates/alz/tests/repository_git.rs (17 tests; real git, isolated HOME and GIT_CONFIG_GLOBAL/NOSYSTEM; stand-in git on PATH swaps a path or records the environment). The tests sit in the alz crate, not alz-io/tests, because the runner starts git through the alz binary (Launcher) and alz-io has none; alz's clippy.toml bans fs and process, so tests use ScratchDir and assert_cmd. AC1: tests a_root_below_the_top_level..., the_gitfile_of_a_linked_worktree..., a_linked_git_folder..., a_link_at_the_config_file..., a_relative_link..., a_pipe_at_the_config_file... each assert the place or Blocked result and that the file the link names (or the repo config) is byte-unchanged. AC2: only_the_missing_keys_are_added..., a_write_never_touches_the_global..., an_include_that_overrides... (verdicts differ and config_answer names override.conf after the re-read). AC3: variables_that_point_at_another_repository... (fake git records the env of every repository call: GIT_DIR, GIT_WORK_TREE, GIT_COMMON_DIR, GIT_CONFIG, GIT_INDEX_FILE absent; GIT_AUTHOR_NAME, GIT_CONFIG_COUNT, GIT_CONFIG_GLOBAL, GIT_CONFIG_NOSYSTEM present; other repo config unchanged, no stray files). Check after write: three tests (link at config, .git swapped for a link, config turned into a pipe) give Stopped::Tampered and no second key; mutation check: with verify disabled those three fail. Not covered: Stopped::Refused (needs a swap between verify and the next seal).

Verified by the lead (2026-11-24): mise run rust:ci and mise run lint exit 0, src/ unchanged. Tests live in crates/alz/tests/repository_git.rs because the runner starts git through the alz binary. The link swapped in between the check and the write can receive the first key; the check after stops the second (the spec's Limit).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
RepositoryGit in alz-io runs only the §9.4 git calls (argument arrays, --end-of-options, every repository variable dropped, a 64 KiB cap), and writes missing identity keys with git config --file only between a no-follow check of .git and .git/config before and after each write; a change between them stops the setup.
<!-- SECTION:FINAL_SUMMARY:END -->
