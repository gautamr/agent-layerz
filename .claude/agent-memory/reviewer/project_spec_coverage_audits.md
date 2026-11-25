---
name: spec-coverage-audits
description: How to audit docs/spec-coverage.md rows; rows often omit tests that exist, name tests under the wrong anchor, and spec message strings are often asserted nowhere
metadata:
  type: project
---

When auditing a `covered` row of docs/spec-coverage.md, sort each weakness into one of two kinds: a rule with no test
anywhere in crates/, or a test that exists but the row does not name it. The 2026-11-25 audits of §3 to §6 and of
§7 to §9 found both kinds; in §7-§9 every named test existed, and most gaps were unnamed tests.

Other patterns from §7-§9:
- Table-driven tests that iterate the table itself (every row of `native_keys::KEYS` is refused) cannot catch a row
  missing from the table; only a spec-list-vs-table test can.
- Tests named under the wrong anchor (typed-table tests under "Security-relevant native keys" and "Facts",
  first-class-key tests under "Reserved keys").
- Test names that overclaim ("every loosening is reported" with one loosening in the input).
- A bold-lead anchor runs to the next anchor, so unlabelled paragraphs fall under the previous anchor.
- Spec message text can drift from code (§7.10 `'paths' is required` vs code `paths is required`).

**Why:** the guard test only checks that the named tests exist, not that they cover the anchor. Exact diagnostic
messages (for example the CollisionReason Display texts in render/write_plan/plan.rs) were asserted by no test even
though the rows were marked covered.

**How to apply:** list the `fn` names per test file first, then read only suspicious bodies; grep crates/ for the
spec's message text or example input before you call a rule untested, and report the unnamed test by path::name so
the caller can add it to the row. See also [[description-bullets-vs-acs]] and [[project-review-tooling]].
