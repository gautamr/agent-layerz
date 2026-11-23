---
id: TASK-55.3.8
title: >-
  R2.8: alz's YAML emitter, byte for byte against golden samples from the
  reference
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
updated_date: '2026-11-23T06:44:00Z'
labels:
  - area:rust
  - area:render
milestone: m-7
dependencies:
  - TASK-55.3.4
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 97390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. decision-28 "Generated YAML": a small alz-owned emitter, tested byte for byte against the TypeScript output, whose samples become golden tests in R2. The spike's 229 samples lived in a temporary scratchpad and are gone (TASK-55.1.1 notes), so this step makes new ones.

- Generator: `crates/alz-core/tools/yaml-emit-samples.mjs` (Node, like `crates/alz/tools/untrusted-code-points.mjs`; it reads `src/` and node_modules and never changes them) calls the reference's `stringifyYaml` (`src/core/yaml.ts`: lineWidth 0, minContentWidth 0) on a corpus of what alz writes: agent, rule and skill frontmatter maps, nested native maps, lists, booleans, integers, empty collections, the allowed.yaml list with its comment, and the scalars the spike listed (yes, no, on, off, 1_000, 1.0.0, dates, 12:30, "true", "123", "0x10", ".inf", "~", "", multi-line text with zero, one and two final line feeds and with leading spaces, control characters, non-ASCII, U+2028, keys that need quoting). It writes `crates/alz-core/tests/fixtures/yaml-emit/<case>.json` (input) and `<case>.yaml` (expected), committed. Keys that are whole numbers are left out and listed for TASK-66 (R5).
- Emitter: `crates/alz-core/src/render/yaml.rs`; input an `EmitValue` (ordered map with text keys, lists, text, booleans, integers), output a `String`. Pure and total.
- The quoting rule it implements goes into the spec: written by the architect from the samples (comment on R2.1, or a spec follow-up task if R2.1 is closed).

Laws (proptest): `yaml::parse(emit(v))` gives back `v` through R2.4's parser; emit is deterministic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every committed sample is reproduced byte for byte (mise run rust:check), and the notes list the corpus by scalar class
- [x] #2 The round-trip property test emit then parse passes
- [x] #3 The quoting rule, with the samples that show it, is in the spec or handed to the architect in a comment
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-23 architect, AC #3 (spec text written, not checked, not committed): new docs/LAYERING-SPEC.md §10.2 'Generated YAML', after §10.1, plus one line in §10.1 after the three untrusted-text classes pointing to it. §10.2 states: the values (no null, float, anchor, tag, comment); layout (two-space indent, [] and {}, nested collections, explicit '? key' past 1024 UTF-16 units of the written key, continuation indent); the text rules 1 to 8 in order (double-quoted for refused characters; quoted for U+FEFF start and blank multi-line; keys with a line feed; the plain-style test with its character lists; literal block; top-level keys starting with --- or ...; core-schema null, bool, int and float forms with regular expressions, plus alz's extra integer forms; plain); single versus double quotes; the double-quoted algorithm from J (JSON form), its escape table and the 40-unit line-break rule; the literal block header (indicator 2, chomping - / + / clip) and line indenting; the read-back law and where refused characters can still come from; and reference drift (decision-30, TASK-73). Top-level scalars: alz never writes one, stated, so no rule is given for one (the force_indent and empty-indent branches of text.rs only serve that case).

Point 3 decided: (a) the emitter escapes U+007F to U+009F (\x7f to \x9f, \N for U+0085) and double-quotes and escapes U+2028 and U+2029 (\L, \P). (b) does not hold: such text cannot come from YAML alz reads, but it can come from allowed.yaml's root and source (file-system real paths), from JSON strings that alz build --adopt converts (settings.json, .mcp.json into settings.yaml, permissions.yaml, hooks.yaml, mcp.yaml), and from file names: adopt names an adopted agent after .claude/agents/<name>.md (src/adapters/claude/adopt-agent.ts line 16). Escaping does not make them round-trip, since alz's reader refuses them decoded too; it keeps them out of committed bytes, where a diff may show NEL, U+2028 or U+2029 as a line break. Planned under TASK-73, marked so in §10.2.

Found: the reference does not round-trip long double-quoted text with a space between a line break and a line feed (writes \\ then a space, reads back a backslash; shown with node and yaml 2.9.1). By reading text.rs, the committed emitter does the same. This breaks this task's round-trip law (AC #2) for such input if the proptest reaches it. Spec §10.2 states alz writes one \ escape there (planned, TASK-73 AC #2); the owner may prefer to fix it in this task instead.

Drift task: TASK-73 (discovered, area:rust, area:spec, m-7, no dependencies, closes with R5's output parity). Version: no bump now, since the Rust alz is unreleased; generated bytes change only for the texts named, and §10.2 says the R8 cutover release lists them.

Lead (2026-11-23): §10.2 'Generated YAML' accepted, including option (a): the emitter escapes U+007F–U+009F (\x7f…\x9f, \N for U+0085) and double-quotes U+2028/U+2029 as \L/\P. Fix both that and the long double-quoted line-break read-back bug (TASK-73 notes) inside R2.8 now, not later: the round-trip law is AC #2. After the fix, §10.2 states them as the Rust rule (drift from the reference, TASK-73) rather than 'planned'. Done after R2.7 to avoid concurrent builds.

Emitter fix (2026-11-23), done in an isolated worktree at 5d3400a and merged by the lead; rust:ci and lint exit 0 there. Case 2 (read-back): tests a_space_between_two_line_feeds_is_written_as_one_escaped_space and ..._reads_back_as_written failed first (\\ written, read back as a backslash) and pass after the fix in double_quoted. Case 1 (escapes): U+007F→\x7f, U+0080→\x80, U+0085→\N, U+009F→\x9f, U+2028→\L, U+2029→\P, as value, item and key; U+00A0 stays plain. Golden samples regenerated: 871, of which 121 are drift (was 52; the earlier '142' count was a misreading): integer-forms-only-alz-reads 30, byte-order-mark-first 12, blank-multi-line 12, escaped-characters-the-reference-writes-raw 54, space-between-line-feeds-in-long-double-quoted-text 13. New proptests reach both cases. Spec §10.2 states both as the Rust rule (drift, TASK-73); nothing 'planned' left. AC #3 (quoting rule in the spec) is §10.2.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz's YAML emitter writes the YAML tree with the reference's rules and reproduces every golden sample from the TypeScript stringifyYaml byte for byte, except 121 recorded drift samples where the reference's output would read back as another value or holds characters alz's reader refuses: alz quotes or escapes those (spec §10.2, TASK-73). Emit-then-parse gives the value back (property tests).
<!-- SECTION:FINAL_SUMMARY:END -->
