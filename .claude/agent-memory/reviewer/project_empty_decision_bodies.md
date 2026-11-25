---
name: empty-decision-bodies
description: Backlog decision files can be created with only a title and empty Context/Decision/Consequences sections; open the file, not just the title
metadata:
  type: project
---

`backlog decision create` writes a file whose Context, Decision and Consequences sections are empty unless the author fills them in. In the TASK-50.9 review (2026-11-22), decision-18 had only a long title. The owner's reasoning was in the task's comments and notes, not in the decision. It happened again in the TASK-50.10 review the same day: decision-19 (the root walk) had an empty body. Decisions 15 to 18 were later filled in by an owner-approved one-off edit (commit "fill the bodies of decisions 15 to 18").

**Why:** a task plan that says "decision-N records the owner's answer" is only true if the body holds that answer, with its options and consequences.

**How to apply:** for every decision a change cites, read the body. Flag empty sections as a docs finding. The fix goes through the backlog CLI or an owner-approved edit, not a hand edit. See [[review-tooling-no-shell]].
