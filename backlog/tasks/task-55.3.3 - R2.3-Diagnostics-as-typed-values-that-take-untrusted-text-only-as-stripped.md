---
id: TASK-55.3.3
title: 'R2.3: Diagnostics as typed values that take untrusted text only as stripped'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T05:17:00Z'
updated_date: '2026-11-23T05:49:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.3.1
  - TASK-55.3.2
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 92390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. `crates/alz-core/src/diagnostic.rs` (doc-6): values and their `Display`, no printing. The aim of doc-6 §2: a diagnostic accepts untrusted text only through the text classes, so a message that holds raw input does not compile.

Types:
- `Level { Error, Warning }`.
- `Code`: an enum of the codes R2 emits as R2.1 names them (`yaml`, `yaml-tag`, `layer`, `manifest`, `lock`, `no-manifest` and the read errors), with `as_str` in kebab-case; an exhaustive match makes a new code a compile error wherever it must be handled.
- `Where`: a layer file (`<layer>:<path>`), a repository file, or a path the person typed (stripped), with an optional `Position { line, column }`. `LayerName` arrives in R2.5; until then `Where` holds the layer part as `Stripped` and R2.5 narrows it.
- `Message`: built only from `&'static str` text and parts that are safe by construction: `Stripped`, `QuotedKey`, `Escaped`, whole numbers, and values of a sealed trait `Printable` implemented only for validated domain types whose alphabet is printable ASCII (TargetName, AccountName, LayerName, GitRef as they land). No `From<String>`.
- `Diagnostic { level, code, at: Option<Where>, message }` and `Diagnostics` (ordered, `has_errors`, counts), `Display` giving `<level> [<code>] <where>: <message>`.
- One error type per boundary converts into diagnostics with `From`; `thiserror` for the error enums (decision-31).

Binary: one renderer in `crates/alz/src/report.rs` writes diagnostics to stderr and maps a run to its exit code; clap rendering is unchanged.

Laws: for any strings put into any parts, the rendered line contains no control or format character and no line feed (proptest); the rendered line matches the spec's grammar.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A diagnostic renders as '<level> [<code>] <where>: <message>', and without '<where>: ' when it has no location (unit test per shape)
- [x] #2 Property test: no rendered diagnostic contains a control or format character or a line feed, whatever strings went into its parts
- [x] #3 A compile_fail doc test shows that a String cannot become a Message part
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R2.2 (2026-11-23): decide and specify whether a single quote inside a QuotedKey is escaped (spec §10.1 is silent), so 'a'b' cannot be misread.

R2.3 implementation (not committed). Files: crates/alz-core/src/diagnostic.rs (new), crates/alz-core/src/lib.rs, crates/alz-core/src/untrusted_text.rs, crates/alz/src/report.rs, docs/LAYERING-SPEC.md §10.1.
Design: Level, Code (Yaml, YamlTag, Layer, Manifest, NoManifest, Lock; later phases add Source, Cache, unreadable etc., the exhaustive as_str match forces handling), Position (NonZeroU32 line/column), Place (LayerFile|RepositoryFile|Given, all Stripped until R2.5/R2.6 narrow them), Where, sealed Printable (no implementor yet; LayerName/TargetName/AccountName/GitRef add one as they land) + MessagePart (private Piece enum; only From<&'static str>, Stripped, QuotedKey, Escaped, u32, u64, usize, and MessagePart::printable; literals are stripped too), Message (new/then), Diagnostic (error/warning), Diagnostics (with/merge/iter/has_errors/error_count/warning_count, Display one line each). thiserror NOT added: R2.3 has no error enum; the boundary errors with From into Diagnostic arrive with R2.4+.
Binary: report.rs emit_diagnostics(&Diagnostics, stderr) -> exit code (0 none/warnings only, 1 any error, write failure 1); dead_code expected outside tests until alz lint (R2.10) calls it.
QuotedKey decision: a single quote inside the key is doubled ('it''s'), as in a YAML single-quoted scalar, so a key cannot close its quotes and forge message text; a backslash is left alone; the 64 cut counts the key's own code points before doubling (never splits a pair). Spec line added in §10.1 Schema errors. Tests: unit tests for doubling, backslash, cut, and the proptest now checks quotes come in pairs.
AC1: diagnostic.rs tests an_error_without_a_location_has_no_where, a_warning_in_a_repository_file_names_the_path, an_error_in_a_layer_file_prints_the_layer_and_the_path, a_yaml_error_adds_the_line_and_the_column, a_path_the_person_typed_is_printed_as_given_with_its_position; proptest a_rendered_diagnostic_starts_with_its_level_and_code; report.rs tests for the stderr lines.
AC2: proptest a_rendered_diagnostic_holds_no_control_or_format_character_or_line_feed (any String into Stripped, QuotedKey, Escaped, Where parts, numbers; the line is a fixed point of Stripped and has no char::is_control, no LF/CR/U+2028/U+2029) and rendered_diagnostics_are_one_line_each.
AC3: two compile_fail doc tests on Message (String into Message::then and Message::new), with a passing twin doc test.
Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, 39 alz-core unit + 17 alz unit + 9 alz integration tests, 3 doc tests, doc -D warnings, msrv, deny, audit). mise run test not run (test/ untouched).

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (68 tests in all); src/ and test/ unchanged. The QuotedKey quote rule (a single quote is doubled) is in spec §10.1. thiserror (decision-31) is not added yet: R2.3 has no error enum; R2.4 adds it with the first boundary error.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Diagnostics are typed values in alz-core: Level, Code (the six R2 codes), Place/Where with non-zero positions, and a Message that only accepts literals, stripped or escaped text, quoted keys, numbers and sealed printable domain values, so raw input cannot reach a diagnostic (compile_fail doc tests). A property test shows every rendered diagnostic is one line with no control or format character. The binary renders them to stderr and derives the exit code. A quoted key doubles a single quote (spec §10.1).
<!-- SECTION:FINAL_SUMMARY:END -->
