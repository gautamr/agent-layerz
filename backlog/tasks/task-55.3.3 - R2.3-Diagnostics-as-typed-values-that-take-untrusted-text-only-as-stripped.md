---
id: TASK-55.3.3
title: 'R2.3: Diagnostics as typed values that take untrusted text only as stripped'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T05:17:00Z'
updated_date: '2026-11-23T05:45:00Z'
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
- [ ] #1 A diagnostic renders as '<level> [<code>] <where>: <message>', and without '<where>: ' when it has no location (unit test per shape)
- [ ] #2 Property test: no rendered diagnostic contains a control or format character or a line feed, whatever strings went into its parts
- [ ] #3 A compile_fail doc test shows that a String cannot become a Message part
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R2.2 (2026-11-23): decide and specify whether a single quote inside a QuotedKey is escaped (spec §10.1 is silent), so 'a'b' cannot be misread.
<!-- SECTION:NOTES:END -->
