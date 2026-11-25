---
name: spec-edits-while-planning
description: While planning, only editorial spec fixes unless the caller asks for spec text; then edit uncommitted on the task branch so it ships with the code; never at plan time on main
metadata:
  type: feedback
---

When planning a task, edit `docs/LAYERING-SPEC.md` only for editorial fixes. Write every behavioural spec
change (new rules, owner decisions from task notes, resolutions of ambiguities) into the plan as a
"Spec changes in the same PR" section, and the implementation PR applies them.

**Why:** the coordinator asked for this on 2026-11-22 (TASK-50.10). The spec has to change in the same
pull request as the behaviour (AGENTS.md "Map"), so a spec edit at plan time would describe code that
does not exist yet.

**How to apply:** for each ambiguity, give a proposed resolution in the plan. Put the ones the owner should
confirm in a task comment with a default and a recommendation, so implementation is not blocked. See
also [[backlog-decision-cli-title-only]].

Exception (coordinator, 2026-11-23, TASK-64): when the brief asks the architect to write the spec text (and a changelog entry) at plan time, edit `docs/LAYERING-SPEC.md` and `CHANGELOG.md` on the task's branch without committing, so they land in the implementation PR; tests-to-be-written (a §13 entry) still go in the plan. Say in the task notes that the edits are on the branch.

Codes and texts (lead, R6.2, 2026-11-24): diagnostic codes are output, so where the spec is silent or coarse, follow the
reference's codes for parity unless a recorded reason says otherwise; do not invent coarser code families. A check that
finds a write already went where it must not is an error, not a warning, even in setup commands.
