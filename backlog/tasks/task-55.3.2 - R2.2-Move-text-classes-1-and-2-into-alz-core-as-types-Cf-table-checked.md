---
id: TASK-55.3.2
title: 'R2.2: Move text classes 1 and 2 into alz-core as types, Cf table checked'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T05:17:00Z'
updated_date: '2026-11-23T05:27:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies: []
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 91390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. decision-26 (amended) moves the stripped and escaped classes into `alz-core`, so diagnostics, YAML errors and the binary share one implementation. Module `crates/alz-core/src/untrusted_text.rs` with `untrusted_text/format_characters.rs`.

Types (private fields, one constructor each):
- `Stripped` (class 1): text with no character of U+0000-U+001F, U+007F-U+009F, U+2028, U+2029 or category Cf. `Stripped::from_untrusted(&str)`, `as_str`, `Display`.
- `Escaped` (class 2): quoted; printable ASCII stays, `"` and `\` get a backslash, every other character is `\u{XXXX}`, upper-case hex, at least four digits. `Escaped::from_untrusted(&str)`.
- `QuotedKey`: a key name from the input for a diagnostic (decision-28): stripped, cut after 64 code points counted after stripping, then `…` (owner answer 1), printed in single quotes.

The Cf table names its Unicode version, 17.0, the one Node 22's \p{Cf} uses, so parity with the reference holds. A unit test reads a vendored extract of UCD 17.0 `extracted/DerivedGeneralCategory.txt` (the Cf lines plus the file header with its Unicode licence notice) under `crates/alz-core/tests/fixtures/unicode/` and checks the table equals it; on a difference it prints the ranges to paste, so no generator is needed. The reference parity test and its fixture move with the table; `crates/alz/tools/untrusted-code-points.mjs` stays only to regenerate that fixture.

The binary's `crates/alz/src/terminal_text.rs` keeps class 3 only (clap's rendering, its own line feeds) and calls `Stripped` for each argument fragment.

Laws (proptest as a dev-dependency, decision-35): `Stripped` output holds no control or format character for any input, and stripping is idempotent; `Escaped` output is printable ASCII and two different inputs never give the same output; `QuotedKey` holds at most 64 code points, plus `…` when cut, between its quotes.

Untrusted input: any text. Out of scope: diagnostics (R2.3).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz-core exports Stripped, Escaped and QuotedKey with private fields, and crates/alz/src/terminal_text/ no longer holds a table (review of the diff)
- [ ] #2 The Cf table test against the vendored UCD 17.0 extract and the parity test against the reference fixture pass (mise run rust:check)
- [ ] #3 Property tests state the three laws and pass, and test 31's control-character case still passes against ALZ_BIN=target/release/alz (case run by name, recorded in the notes)
<!-- AC:END -->
