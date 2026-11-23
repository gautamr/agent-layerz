---
id: TASK-55.4.1
title: 'R3.1: Spec: git subprocesses, cache location and account-folder refusals'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T07:41:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies: []
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 102390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (TASK-55.4 plan, "Spec changes in the same PR"). Writes the spec text that R3's behaviour follows, after the owner has answered Q1, Q2, Q4, Q5 and Q6 in TASK-55.4's notes. It changes no code.
- §6 gets a new paragraph, "Git subprocesses":
  - argument arrays, `--end-of-options` before revisions, `--` only before paths;
  - the child environment: the eight names TASK-63 lists, removed for every cache and clone operation, and GIT_CONFIG_* per Q2; GIT_TERMINAL_PROMPT=0; the guard variables;
  - the git floor and the error an older git gives (Q4);
  - no controlling terminal (Q1; TASK-70 AC #1);
  - the deadline (Q5);
  - the TypeScript reference inheriting the hook environment, as drift.
- §6 cache location: an empty or relative XDG_CACHE_HOME counts as unset for every command (TASK-53). Add the Q6 answers: a cache inside the repository, a link in the root's path, a missing root created with mode 0700.
- §6 and §9.1: every command that resolves the stack refuses, before it opens a file there, an unpinned `path:` layer whose real directory is `$HOME` or above, lies in an account folder or holds one, and a cache root in an account folder (TASK-52). Give the code and the message, and the reference's behaviour as drift.
- §7.13 "No fetch on resolve" and §9.1 (TASK-55.4 AC #3): replace the realpath-and-lstat wording with the component walk (decision-27), and list the reference's realpath behaviour as drift.
- §13: an entry for each new acceptance case R3.11 adds.
- A CHANGELOG line under the unreleased Rust cutover, listing the changes users will see. No version bump.
Refs: decision-24, decision-27, decision-30, TASK-52, TASK-53, TASK-63, TASK-70.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Spec §6 states the git child environment with the eight removed names, the floor and what an older git gives, no controlling terminal and the deadline (spec diff; TASK-63 AC #3, TASK-70 AC #1)
- [ ] #2 Spec §6 and §9.1 state the cache location rule for every command, and the build-side refusal of a layer or cache root in an account folder, each with its code and message (spec diff; TASK-52, TASK-53)
- [ ] #3 The Node-era realpath wording in §7.13 and §9.1 is replaced by the component walk, and the reference's behaviour is listed as drift (spec diff; TASK-55.4 AC #3)
<!-- AC:END -->
