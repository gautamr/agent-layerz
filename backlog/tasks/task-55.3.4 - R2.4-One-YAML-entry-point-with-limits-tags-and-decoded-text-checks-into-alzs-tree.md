---
id: TASK-55.3.4
title: >-
  R2.4: One YAML entry point with limits, tags and decoded-text checks into
  alz's tree
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
  - TASK-55.3.3
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 93390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. decision-28 (amended), spec §7.13 "YAML limits" (TASK-65) and R2.1. Module `crates/alz-core/src/yaml.rs` with `yaml/{text,events,tree,load,error,fields}.rs`. Adds `serde-saphyr =2.2.0` (`default-features = false`, `features = ["deserialize"]`, decision-28) and `serde` without derive (decision-32). Licence: serde-saphyr's `deserialize` feature brings in encoding_rs, licensed `(Apache-2.0 OR MIT) AND BSD-3-Clause`, which `deny.toml` does not allow yet (scratch graph, 2026-11-23); `mise run rust:deny` passes only after the owner decides whether to allow BSD-3-Clause (TASK-55.3 Q7).

Steps, each a pure function:
1. `YamlText::from_bytes(&[u8])`: at most 1 MiB (1048576 bytes pass, one more fails), strict UTF-8 with the error position, and the raw control-character set: tab, line feed and carriage return are allowed, every other character of U+0000-U+001F, U+007F-U+009F, U+2028 and U+2029 is refused (decision-28 amended 2026-11-23). The only way to get a `YamlText`; R4 adds a constructor for a frontmatter slice of a capped file, so the cap counts the whole file by type.
2. Event pre-pass over `serde_saphyr::granit_parser` (re-exported with `deserialize`; read in the 2.2.0 source: node events carry `Option<Tag>`, aliases refer to anchors by id; not run yet, so the first test pins it). This is decision-28's open choice of how tags reach alz: the pre-pass, which also counts what §7.13 defines. It refuses explicit tags outside `!!str !!int !!float !!bool !!null !!map !!seq` (`yaml-tag`), a second document, an alias to an anchor whose node is still open (cyclic, TASK-60) or to no anchor (unresolved, TASK-59), and counts the four budgets as §7.13 defines them: aliases as written at most 100, nesting at most 64 (`a: [x]` is two), nodes at most 20000, replayed events at most 10000 (an alias costs its anchor's events). It checks every decoded scalar, keys included, against the decoded-text set: only tab and line feed are allowed, so a `\r` escape is refused.
3. `serde_saphyr` load with decision-28's options (strict booleans, merge keys as ordinary keys, duplicate keys an error, no legacy octal, no snippet, non-finite floats passed through, underscored numbers as strings, the four budgets) into alz's tree through an alz `Visitor` on `deserialize_any`. Its budgets are defence in depth; if one refuses a document the pre-pass accepts at a spec boundary, raise only that option to the smallest value that accepts the boundary and record it in the notes.
4. Tree: `Node { value, at: Position }`, `Value = Null | Bool | Int(i128) | Float(f64) | Text(String) | Seq(Vec<Node>) | Map(Mapping)`. `Mapping` keeps keys in document order (TASK-66 decides output order in R5), unique, scalar keys only.
5. `yaml/fields.rs`, the helpers every schema uses: unknown keys reported as `QuotedKey`s; `text(field)` refuses a number and suggests quoting; `whole_number(field)` counts a float without a fraction; `text_list(field)`. Errors carry the key path, never the value.

`YamlError` is an enum of kinds with a `Position`, converting into a `Diagnostic` with code `yaml` or `yaml-tag`, holding no input text except one `QuotedKey` (a duplicate key). serde-saphyr's `Display` text is never read. `crates/alz-core/clippy.toml` disallows serde-saphyr's parse functions, with one `#[expect]` in `yaml/load.rs`, so this is the single entry point every YAML file goes through (TASK-62 AC #2).

Tests: a pair at each limit (100 and 101 aliases, 64 and 65 levels, 20000 and 20001 nodes, 10000 and 10001 replayed events, 1 MiB and 1 MiB + 1 byte); each tag decision-28 names (`!!binary`, `!!timestamp`, `!!set`, `!!omap`, `!!merge`, `!!value`, `!foo`, `!!python/object`) refused and the core tags accepted; `"\x07"`, `"\e"` and `" "` in double quotes refused; two documents; `&a [*a]`; `*marker` unresolved, with the marker absent from the diagnostic; CRLF line ends parse while a `\r` escape is refused; `017`, `0b101`, 2^53 + 1, `"true"` staying text, `1e3` as a whole number. Property test (decision-28 Consequences): the diagnostic of any rejected input holds no input text except one quoted, stripped key name. An alias bomb at 1 MiB is refused quickly (time recorded in the notes).

Untrusted input: every byte.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 yaml::parse is the only code in the workspace that calls serde-saphyr (clippy disallowed-methods with one #[expect], shown in the PR) and it applies the 1 MiB cap, strict UTF-8 and the four budgets to every input (TASK-62 AC #2)
- [ ] #2 For each of the four budgets and the cap, a document at the limit parses and one past it is refused, counted as spec §7.13 defines (unit tests)
- [ ] #3 Every tag decision-28 lists is refused with yaml-tag and the core tags are accepted; two documents, a cyclic alias, an unresolved alias and an escape that decodes to a forbidden control character are refused, and no diagnostic holds the alias name (unit tests)
- [ ] #4 Property test: the diagnostic of any rejected input holds no input text except one quoted, stripped key name
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23): deny.toml gets a licence exception for encoding_rs only (BSD-3-Clause), added with serde-saphyr in this subtask; decision-28 amended.
<!-- SECTION:NOTES:END -->
