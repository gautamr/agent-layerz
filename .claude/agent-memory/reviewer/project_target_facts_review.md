---
name: target-facts-review
description: How to review target-facts.md rows - surfaces the vendor docs qualify (VS Code, cloud, forks, plan+auto) get dropped; mixed rows get the strongest status
metadata:
  type: project
---

In target-facts rows, the Status cell is the last table cell, and the harness
(`crates/alz-test-support/src/facts/record.rs`, `is_unverified`) counts every row as verified
unless that cell ends in UNVERIFIED. So a DOCS row counts as recorded, and a row that mixes a
`-p` SMOKE probe with interactive DOCS or inference tends to be labelled SMOKE.

**Why:** in the TASK-114 review (2026-11-27), three rows were labelled SMOKE although the
interactive or "leaving" half they claimed was DOCS-only or plain inference. Qualifiers the
Claude docs state were also dropped: VS Code never reads a project `defaultMode`, cloud ignores
`dontAsk` from settings, forks keep `AskUserQuestion`, plan mode lets the classifier approve
commands when auto mode is available, and some sandboxed commands (`FOO=bar cmd`, `eval`,
Monitor) skip auto-allow.

**How to apply:** if the caller downloaded the vendor pages, grep each row's claim against
them and look for "only", "every" and "never". Ask for rows to be split by surface (`-p` or
interactive), as CF1 is, rather than given a combined status. See also
[[review-tooling-no-shell]].
