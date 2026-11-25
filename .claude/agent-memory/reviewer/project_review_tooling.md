---
name: review-tooling-no-shell
description: The reviewer subagent has only Read/Grep/Glob (no Bash), so it cannot run git diff, tests or lint; review uncommitted work by reading files and rely on the caller for check results
metadata:
  type: project
---

The reviewer runs without a shell: no `git diff`, `mise run test`, `lint` or `alz:check`.

**Why:** Observed during the TASK-34 review (2026-11-21); the caller's request assumed `git diff HEAD`, but the tool set had no Bash.

**How to apply:** For modified files, read the current file and infer the change. Use the git status snapshot to confirm that no existing test or `expected/` file changed. Say in the report that results come from reading, not from running anything. Take test/lint results only from the caller's verified claims, and attribute them to the caller.

Workarounds found (R0 review, 2026-11-22): commit subjects and authors of a branch can be read from `.git/logs/HEAD` with Grep (pattern `.`); `backlog task view` is not available, so read `backlog/tasks/*.md` and `backlog/decisions/*.md` directly.
