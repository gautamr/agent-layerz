---
name: record-drafts-early
description: On drafting tasks and spikes, write findings into the task notes after each batch and run what the acceptance criteria need first; the turn limit cut two sessions short (TASK-55.1.2, TASK-55.1.1)
metadata:
  type: feedback
---

When a task asks for several drafts (options, trade-offs, a recommendation), append each draft to the task as soon as it is ready. Do not finish all the research first. Mark anything not checked as UNVERIFIED instead of researching it further.

**Why:** on 2026-11-22 (TASK-55.1.2, the R0 choices) the coordinator stopped the session at its turn limit, while it was still probing crates and clap, and nothing had been recorded yet.

**How to apply:** budget about one verification pass per choice. Once a choice has its options and a recommendation, write it with `backlog task edit --append-notes`. Spend only the turns that are left on extra checks. See [[fact-checks-without-account-folders]] for where to put scratch output.

It happened again on TASK-55.1.1 (YAML spike, 2026-11-22). The TypeScript reference had been measured, but the Rust comparison crate, which AC #1 asks to run, had not been built before the limit. In a spike, run the evidence the acceptance criteria name before broad source reading, and append notes after each batch.
