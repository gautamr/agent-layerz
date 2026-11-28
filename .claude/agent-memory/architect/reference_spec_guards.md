---
name: spec-guards
description: How the two spec guards read LAYERING-SPEC.md (spec_coverage anchors, diagnostic_codes tables) and the edits that break them
metadata:
  type: reference
---

Two tests parse `docs/LAYERING-SPEC.md` directly; spec edits must respect both.

- `crates/alz-test-support/tests/spec_coverage.rs`: any line that starts with `**` outside a code
  fence is a bold-lead anchor `§<section> <text between first two **>`. A wrapped cross-reference
  such as a line beginning `**Writing.**)` becomes a phantom anchor. Rewrap so it is mid-line.
  Each new anchor needs a row in `docs/spec-coverage.md`; `planned` rows must contain `TASK-`.
  The guard prints the expected Summary table on failure, so copy the numbers from there.
- `crates/alz/tests/diagnostic_codes.rs`: reads every line starting with "| `" between
  `**Merge diagnostics**` and `**Exit codes.**`. Every code there needs a case or a
  `NOT_REACHED_HERE` entry, and a second level in the Level column needs its own case. A code
  that cannot be produced yet breaks `rust:check`. At plan time, reuse an existing code (as
  TASK-104 did with `target`) or give a test task the new code.

**How to apply:** run `cargo test -p alz-test-support --test spec_coverage` after every spec edit,
before the full `mise run rust:check` (about 4 minutes). See [[backlog-decisions]] for CLI limits.
