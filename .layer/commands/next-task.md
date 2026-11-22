---
description: Pick the next unblocked Backlog.md task and route it to the right subagent.
argument_hint: "[task-id]"
---
Work on Backlog.md task $ARGUMENTS, or the next one if none is given.

1. Run `backlog instructions task-execution` if you have not read it in this session.
2. Without an ID: list `To Do` tasks (`backlog task list --status "To Do" --plain`), take the
   earliest milestone first, skip tasks whose dependencies are not Done and tasks labelled
   `owner:human`, then the highest priority.
3. Read the task (`backlog task view <id> --plain`). Labelled `owner:human`: stop and tell
   the owner what to do; agents cannot do it.
4. Route by label:
   - no plan yet, or `area:spec` → architect
   - `area:ci`, `area:release` → release-engineer
   - `area:docs` → docs-writer
   - otherwise → implementer
5. Review once per unit of work: a task without subtasks, or a parent task once all its
   subtasks are implemented. A subtask gets its checks (tests, lint, `alz:check`) but no review
   of its own; subtasks of one parent share one branch, which is merged only after the parent's
   review. At the end of the unit, run the reviewer on the whole unit's diff, and the
   security-reviewer when any task in it has `area:security` or touches files, subprocesses,
   hooks, permissions or secrets. (TASK-56 makes this cadence a layer setting.)
6. Fix or record every finding, then finish the unit's tasks with
   `backlog instructions task-finalization`.

Report: the task, who did what, the checks run, and anything waiting for the owner.
