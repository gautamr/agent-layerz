---
name: private-security-tasks
description: How to plan the fix of a local-only security task (file in .git/info/exclude, e.g. TASK-67) without leaking it into committed board files, tests or specs
metadata:
  type: feedback
---

A security finding can live in a local-only task whose file `.git/info/exclude` keeps out of git (TASK-67, settings top-level keys, 2026-11-23). Plan its fix on the public board under neutral words only: describe the intended rule ("only known targets at the top level, ordered maps of validated keys"), never the flaw, the exploit or the trigger key. Write the specifics only with `backlog task edit <private-id> --append-notes`.

**Why:** the owner's instruction for TASK-67. The fix must not be visible before the Rust release ships it. The R4 pull request describes it afterwards, and then the private task is committed.

**How to apply:**
- In acceptance tests, put the unusual keys in as ordinary table rows without comment. Use `referenceDrift("<public parent task>", ...)`, never the private ID, because a committed test would point at a task file that is not in git.
- Before handing back, run `grep -il 'proto\|<private-id>\|flaw\|exploit' backlog/tasks/<new files>` and `git check-ignore -v <private file>`.
- If you find a new security-relevant gap while planning, state it neutrally as the intended rule on the board, and give the details only in the handback report. Ask there whether it should be private too.
- Owner questions on the public board count too: never quote a private task's criterion ("describes the flaw") or say that private tasks exist. R6 (2026-11-24): the grep caught it in question 9; the fix was `backlog task edit --notes` with the whole notes text and the question reworded ("the cutover PR describes every security fix it ships"), details only in the handback.
See [[spec-edits-while-planning]].
- Evidence notes must not name the leak grep's terms or say "local-only task IDs": that itself says private
  tasks exist (caught in R6.2, 2026-11-24). Write "Leak grep of the added lines in <files>: no match."
- The leak grep's `proto` matches "delivery-protocol"; read the hit before treating it as a leak.
- After decision-39 (2026-11-24, Rust only, no npm wrapper) there is no "PR that makes the Rust binary
  the default": the public disclosure criterion moved to the first Rust release (TASK-55.8.4, owner
  approves wording and timing). The private tasks' "neutral acceptance rows" criteria pointed at the
  vitest suite, which R8 deletes; their proof must be Rust tests the coverage matrix names (rules stated
  neutrally in the spec). Whether the owner reworded them is his call; check the private notes first.
