---
name: description-bullets-vs-acs
description: Backlog subtasks list required test cases in the Description, not the ACs; ACs get ticked while a bullet is unmet, so check each Description bullet against the tests
metadata:
  type: project
---

In this repo a subtask's Description often lists the concrete cases to add (e.g. R5.15: "a passthrough .claude/Settings.json collides; a protected path in another case is left alone"), while the ACs only say "the writer-rule cases are in the suite". In the R5 review (2026-11-24) the AC was ticked and two of the listed cases did not exist.

**Why:** ACs are coarse and get ticked from the implementer's summary; the Description bullets are the real checklist.

**How to apply:** For every subtask under review, grep the tests for each Description bullet, not just the ACs. Report a missing bullet as a gap against a ticked AC. See also [[review-tooling-no-shell]].
