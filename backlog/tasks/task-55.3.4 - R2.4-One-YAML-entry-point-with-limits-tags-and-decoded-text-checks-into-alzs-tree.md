---
id: TASK-55.3.4
title: >-
  R2.4: One YAML entry point with limits, tags and decoded-text checks into
  alz's tree
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T05:17:00Z'
updated_date: '2026-11-23T06:04:00Z'
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
- [x] #1 yaml::parse is the only code in the workspace that calls serde-saphyr (clippy disallowed-methods with one #[expect], shown in the PR) and it applies the 1 MiB cap, strict UTF-8 and the four budgets to every input (TASK-62 AC #2)
- [x] #2 For each of the four budgets and the cap, a document at the limit parses and one past it is refused, counted as spec §7.13 defines (unit tests)
- [x] #3 Every tag decision-28 lists is refused with yaml-tag and the core tags are accepted; two documents, a cyclic alias, an unresolved alias and an escape that decodes to a forbidden control character are refused, and no diagnostic holds the alias name (unit tests)
- [x] #4 Property test: the diagnostic of any rejected input holds no input text except one quoted, stripped key name
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23): deny.toml gets a licence exception for encoding_rs only (BSD-3-Clause), added with serde-saphyr in this subtask; decision-28 amended.

Implementer (2026-11-23): code done, ACs left unchecked, nothing committed.

Files: Cargo.toml, Cargo.lock, deny.toml, crates/alz-core/{Cargo.toml,clippy.toml}, src/lib.rs (pub mod yaml), src/diagnostic.rs (Position::FIRST added), src/yaml.rs, src/yaml/{text,events,tree,load,error,fields,tests}.rs.
Dependencies (workspace, exact): serde =1.0.229 (default-features=false, std), serde-saphyr =2.2.0 (default-features=false, deserialize), thiserror =2.0.21 (default-features=false, std). deny.toml: [[licenses.exceptions]] encoding_rs allow BSD-3-Clause (only); [[bans.features]] serde deny derive (decision-32).

AC1: crates/alz-core/clippy.toml disallowed-methods lists every public serde_saphyr parse function (from_str*, from_slice*, from_bytes_multiple*, from_multiple*, from_reader*, read*, with_deserializer_from_*). Probe: a call to serde_saphyr::from_str outside load.rs gives 'use of a disallowed method'. The single #[expect(clippy::disallowed_methods)] is on fn read in yaml/load.rs (calls with_deserializer_from_str_with_options). parse = events::check (pre-pass) then read; YamlText::from_bytes is the only constructor of YamlText (cap, UTF-8, raw control set). The pre-pass calls granit_parser (re-exported by serde-saphyr) from yaml/events.rs, which is private; granit_parser's Parser is not in the ban list, so that is a second serde-saphyr touch point inside yaml; a second #[expect] would be needed to ban it.
AC2: yaml::tests: a_document_with_100_aliases_parses_and_101_is_refused, nesting_of_64_levels_..._65, a_mapping_key_and_a_flow_sequence_count_as_two_levels, an_alias_counts_the_levels_of_its_anchor_where_it_is_used, a_document_of_20000_nodes_parses_and_20001_is_refused, replayed_events_of_10000_parse_and_10001_are_refused, an_alias_bomb_that_nests_aliases..., a_document_at_every_limit_at_once_parses_through_serde_saphyrs_own_budgets; yaml::text::tests a_file_of_one_mebibyte_passes_and_one_more_byte_fails and a_scalar_of_one_mebibyte_parses_through_the_whole_entry_point.
AC3: every_tag_outside_the_core_set_is_refused_with_yaml_tag (!!binary, !!timestamp, !!set, !!omap, !!merge, !!value, !foo, !!python/object, ! and a verbatim tag), the_core_tags_are_accepted, more_than_one_document_is_refused, a_cyclic_alias_is_refused (&a [*a] is Alias to an open anchor in granit 2.2.0: pinned), an_unresolved_alias_is_refused_and_its_name_never_reaches_the_diagnostic (line is exactly 'error [yaml] layer.yaml:1:4: an alias whose anchor is not defined'), an_escape_that_decodes_to_a_forbidden_control_character_is_refused (\x07 \e \u2028 \u2029 \N \r \0 \x7f, and a key), crlf_line_ends_parse_and_a_carriage_return_escape_does_not.
AC4: proptest a_rejected_input_leaves_no_input_text_in_its_diagnostic_except_one_quoted_key (fragments carrying a SECRET marker in anchors, aliases, tags, keys, scalars and escapes, plus raw bytes).
Time: an alias bomb of 150000 aliases in under 1 MiB is refused in 0.01 s (debug build, cargo test); the whole alz-core suite runs in 0.6 s.
Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, 79 unit + 3 doc tests, doc, MSRV, deny incl. the encoding_rs exception: advisories ok, bans ok, licenses ok, sources ok; audit). mise run lint ok. mise run test: 1081 passed, 4 expected fail, 1 skipped. No src/ change, so alz:build not needed.

serde-saphyr budgets (decision-28 options all set; Options and Budget are non_exhaustive so set field by field): max_aliases 100, max_depth 64, alias_limits.max_total_replayed_events 10000 as decided. Two differences from the decision's literal list, both needed because serde-saphyr charges replayed events to its node budget and has an extra ratio rule: max_nodes = 20000 + 10000 = 30000 (the pre-pass enforces 20000 as written), and enforce_alias_anchor_ratio = false (a rule outside spec 7.13). The test at every limit at once passes through them. Counting differences to know: (1) the pre-pass counts depth of the expanded tree (alias at depth d of an anchor h deep needs d+h <= 64), which serde-saphyr also does; spec 7.13 does not say whether aliases expand for depth. (2) replayed events: an alias nested inside an anchored node counts 1 + the events of its own anchor in the pre-pass (spec text: each alias is one event); serde-saphyr does not count the nested alias event, so its own limit is never stricter. (3) Duplicate keys: serde-saphyr reports them (policy Error); its key is Option, so DuplicateKey holds Option<QuotedKey>.
Deviation to confirm: decision-32 says no type implements Deserialize. Node positions need serde_saphyr::Spanned<T>, which needs T: Deserialize, so positions come from the pre-pass instead (one position per node in reading order, aliases expanded, each replayed node with the alias position) and are given to the hand-written visitor in the same order. Kept decision-32 literal; the coupling is covered by nodes_carry_the_position_they_were_read_at_and_an_alias_use_its_own.

Lead (2026-11-23): (a) decision-32 stays literal (no type implements Deserialize); node positions come from the pre-pass in reading order, covered by nodes_carry_the_position_they_were_read_at_and_an_alias_use_its_own. (b) The lead also banned the granit parser constructors (Parser::new_from_str, _with_options, new_from_iter*, new_from_fallible_iter*) in crates/alz-core/clippy.toml, with one #[expect] on events::check; the build proves the lint fires there. (c) Spec §7.13 clarified to match the code and serde-saphyr: depth counts the tree as aliases expand it (d + h), and an alias inside a replayed node costs one event plus its anchor's size each replay. (d) serde-saphyr max_nodes is 30000 because it charges replayed events to nodes; the pre-pass enforces 20000 written nodes exactly; enforce_alias_anchor_ratio is off (not a spec rule). Verified: mise run rust:ci exits 0 (108 tests; deny ok with the encoding_rs-only exception); src/ unchanged.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
yaml::parse is the one YAML entry point: a 1 MiB cap and strict UTF-8 (YamlText), an event pre-pass that enforces the spec's four budgets exactly, the tag rule, one document, no cyclic or unresolved alias and the raw/decoded control-character rules, then serde-saphyr into alz's own tree through a hand-written visitor. Clippy bans every other serde-saphyr and granit parser entry. No diagnostic holds input text but one quoted key (property test). Adds serde, serde-saphyr, thiserror and the encoding_rs-only licence exception. Fixes TASK-59, 60 and 62 in Rust (closed in R2.11).
<!-- SECTION:FINAL_SUMMARY:END -->
