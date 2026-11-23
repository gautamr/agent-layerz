---
id: TASK-55.4.1
title: 'R3.1: Spec: git subprocesses, cache location and account-folder refusals'
status: Done
assignee:
  - '@architect'
created_date: '2026-11-23T07:41:00Z'
updated_date: '2026-11-23T08:04:00Z'
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
Plan for this step (TASK-55.4 plan, "Spec changes in the same PR"). Writes the spec text that R3's behaviour follows, following the owner's answers 1, 2, 4, 5 and 6 in TASK-55.4's notes (2026-11-23). It changes no code.
- §6 gets a new paragraph, "Git subprocesses":
  - argument arrays, `--end-of-options` before revisions, `--` only before paths;
  - the child environment: the eight names TASK-63 lists, removed for every cache and clone operation, and GIT_CONFIG_PARAMETERS, GIT_CONFIG_COUNT and every GIT_CONFIG_KEY_n and GIT_CONFIG_VALUE_n (answer 2); GIT_TERMINAL_PROMPT=0; the guard variables;
  - git 2.30 or later, and error [git] naming the found and the minimum version below it (answer 4);
  - no controlling terminal: alz re-runs itself with an internal argument, that child calls setsid() and then runs git; the user's ssh command is untouched and ssh's graphical askpass stays allowed (answer 1; TASK-70 AC #1);
  - deadlines of 10 minutes for a clone and 2 minutes for any other call; on expiry the process group is killed and the fetch fails (answer 5);
  - the TypeScript reference inheriting the hook environment, as drift.
- §6 cache location: an empty or relative XDG_CACHE_HOME counts as unset for every command (TASK-53). Add answer 6: an absolute XDG_CACHE_HOME inside the repository is refused, a link in the root's own path is followed, and a missing root is created with mode 0700.
- §6 and §9.1: every command that resolves the stack refuses, before it opens a file there, an unpinned `path:` layer whose real directory is `$HOME` or above, lies in an account folder or holds one, and a cache root in an account folder (TASK-52). Give the code and the message, and the reference's behaviour as drift.
- §7.13 "No fetch on resolve" and §9.1 (TASK-55.4 AC #3): replace the realpath-and-lstat wording with the component walk (decision-27), and list the reference's realpath behaviour as drift.
- §13: an entry for each new acceptance case R3.11 adds.
- A CHANGELOG line under the unreleased Rust cutover, listing the changes users will see. No version bump.
Refs: decision-24, decision-27, decision-30, TASK-52, TASK-53, TASK-63, TASK-70.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Spec §6 states the git child environment with the eight removed names and the GIT_CONFIG_* variables, git 2.30 or later with error [git] below it, no controlling terminal through the self re-exec and setsid, and deadlines of 10 and 2 minutes (spec diff; TASK-63 AC #3, TASK-70 AC #1)
- [x] #2 Spec §6 and §9.1 state the cache location rules of answer 6 for every command, and the refusal of a layer or cache root in an account folder by every command that resolves the stack, each with its code and message (spec diff; TASK-52, TASK-53)
- [x] #3 The Node-era realpath wording in §7.13 and §9.1 is replaced by the component walk, and the reference's behaviour is listed as drift (spec diff; TASK-55.4 AC #3)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Architect (2026-11-23): the spec text is written on feat/rust-r3-task-55.4 and not committed, so it lands in R3's pull request with the behaviour. Changes in docs/LAYERING-SPEC.md:
- §6 step 1, new 'The layer cache': XDG_CACHE_HOME only when absolute; HOME per §7.13 'Environment', else error [cache] 'cannot locate the layer cache…'; a link in the root's own path followed, walked one component at a time; a root that touches an account folder is refused with error [cache]; an absolute value inside the repository is refused with error [cache]; missing folders get mode 0700; drift (TASK-52, TASK-53).
- §6 step 1, new 'Layers in or around an account folder': the component walk for path: layers and local git: sources; error [source] 'layer … touches an account folder or the home folder; alz never reads there'; openat and fstat reads; realpath drift (TASK-52).
- §6 step 1, new 'Git subprocesses': argument lists, --end-of-options; git 2.30 or later, else error [git] 'alz needs git 2.30 or later; found <version>' ('an unreadable version', 'none'); the environment (the eight names, GIT_CONFIG_PARAMETERS, GIT_CONFIG_COUNT and its KEY_n/VALUE_n removed; GIT_TERMINAL_PROMPT=0; the guards); no terminal through the self re-exec and setsid, the user's ssh command untouched, graphical askpass allowed; deadlines of 600 and 120 seconds with the process group killed; drift (TASK-63, TASK-70). 'A failed fetch' points to it.
- §7.13 'No fetch on resolve': the real path is found one component at a time; reads open files relative to their folder and check with fstat; realpath drift (TASK-55.4 AC #3).
- §9.1: the term 'touches an account folder' (NFC and lowercased, per component), and the build-side refusal next to resolution's miss.
- §13 entry 28, for R3.11's lint twins.
- CHANGELOG.md [Unreleased] Changed: the changes users see when they move to the Rust alz. No version bump, because the layer format, the lock and the generated output do not change.

Decided, to notify: the account-folder refusal covers pinned path: layers and local git: sources too, not only unpinned path: layers, because §9.1 forbids reading there for every source. git that cannot be started gives 'found none'.

Checks: each criterion was verified by reading the edited sections. No test applies to spec text; no code changed. The board has no In Review status, so the task stays In Progress until the owner or a reviewer reviews the text and sets Done.

Lead (2026-11-23): the spec text follows the owner's answers; the account-folder refusal covering pinned path: layers and local git: sources too is accepted (§9.1 forbids reading there for any source). The R3 unit review checks the text against the code.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Spec text for R3 (owner answers 1, 2, 4, 5, 6): §6 'The layer cache', 'Layers in or around an account folder' and 'Git subprocesses'; §7.13 and §9.1 now describe the component walk and the fstat-on-handle reads; §13 entry 28; a CHANGELOG entry. All of it is uncommitted on feat/rust-r3-task-55.4. No code changed and no test applies.
<!-- SECTION:FINAL_SUMMARY:END -->
