---
id: TASK-55.3.2
title: 'R2.2: Move text classes 1 and 2 into alz-core as types, Cf table checked'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T05:17:00Z'
updated_date: '2026-11-23T05:45:00Z'
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
- [x] #1 alz-core exports Stripped, Escaped and QuotedKey with private fields, and crates/alz/src/terminal_text/ no longer holds a table (review of the diff)
- [x] #2 The Cf table test against the vendored UCD 17.0 extract and the parity test against the reference fixture pass (mise run rust:check)
- [x] #3 Property tests state the three laws and pass, and test 31's control-character case still passes against ALZ_BIN=target/release/alz (case run by name, recorded in the notes)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (implementer, not committed).
Files: crates/alz-core/src/untrusted_text.rs (Stripped, Escaped, QuotedKey; private fields, one from_untrusted each; unit and proptest laws), untrusted_text/format_characters.rs (moved with git mv, pub(super), plus the UCD test), tests/fixtures/unicode/DerivedGeneralCategory-17.0.0-Cf.txt (header with licence notice and the 21 Cf lines, unchanged, from unicode.org/Public/17.0.0/ucd/extracted), tests/fixtures/untrusted-code-points.txt (git mv from crates/alz/tests/fixtures); crates/alz/src/terminal_text.rs keeps class 3 only and calls Stripped (the keep-line-feeds guard splits on LF and strips each line); crates/alz/tools/untrusted-code-points.mjs now writes only the parity fixture (path updated); Cargo.toml (workspace deps alz-core and proptest =1.12.0, default-features=false, features=[std], per decision-35), alz-core dev-dependency, alz dependency, Cargo.lock.
AC1: crates/alz/src/terminal_text/ no longer exists; alz-core exports Stripped, Escaped, QuotedKey (lib.rs, one re-export each, fields private).
AC2: cargo test -p alz-core: the_table_equals_the_format_characters_of_unicode_17 and the_unicode_data_holds_170_format_characters pass; mutation check (one range removed) fails with the ranges to paste, restored afterwards. parity_with_the_typescript_reference_over_every_code_point passes. mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, msrv, deny: advisories/bans/licenses/sources ok, audit).
AC3: proptest laws pass: stripped_text_holds_no_control_or_format_character, stripping_is_idempotent, escaped_text_is_printable_ascii, two_different_texts_never_escape_alike, a_quoted_key_holds_at_most_64_code_points_between_its_quotes. After cargo build --release --locked: ALZ_BIN=$PWD/target/release/alz npx vitest run test/acceptance/31-account-which.test.ts -t 'removes control characters from an unknown command' -> 1 passed.
Notes: Escaped and QuotedKey carry their quotes in Display (double and single, per spec 10.1); a single quote inside a key is not escaped (spec says nothing). cargo tree -e normal for alz-core lists no proptest. mise run test not run (test/ untouched). No proptest-regressions files were created.

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (fmt, clippy -D warnings, tests incl. 21 in alz-core, doc, msrv, deny all ok, audit); src/ unchanged; the vendored UCD extract keeps the Unicode licence notice. Open for R2.3: whether a single quote inside a QuotedKey is escaped (spec §10.1 is silent) — R2.3 settles it with the spec line.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Text classes 1 and 2 now live in alz-core as Stripped, Escaped and QuotedKey (private fields, one constructor each); the binary keeps only class 3. The Cf table is checked against the vendored Unicode 17.0 data and against the TypeScript reference over every code point, and proptest laws cover stripping, escaping and the 64-code-point key cut.
<!-- SECTION:FINAL_SUMMARY:END -->
