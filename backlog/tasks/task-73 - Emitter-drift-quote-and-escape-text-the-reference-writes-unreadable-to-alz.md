---
id: TASK-73
title: 'Emitter drift: quote and escape text the reference writes unreadable to alz'
status: To Do
assignee: []
created_date: '2026-11-23T06:27:00Z'
updated_date: '2026-11-23T06:32:00Z'
labels:
  - discovered
  - area:rust
  - area:spec
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
priority: medium
type: bug
ordinal: 101390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while writing the spec text of TASK-55.3.8 (spec §10.2 'Generated YAML'). The Rust YAML emitter differs from the reference's bytes (stringifyYaml in src/core/yaml.ts) on purpose in four cases, each reference drift under decision-30:

1. Integer forms only alz's reader reads: 0b integers with or without a sign, and 0o and 0x integers with a sign. The reference writes them plain and alz's reader reads them back as integers, so alz quotes them.
2. Text that starts with U+FEFF: written plain by the reference and refused by alz's reader; alz quotes it.
3. Text of only spaces and line feeds that holds both: the reference writes a block that reads back without its spaces; alz quotes it.
4. U+007F to U+009F, U+2028 and U+2029: the reference writes U+007F to U+009F raw inside double quotes, and U+2028 and U+2029 raw, plain or in a block. alz double-quotes such text and escapes them (\x7f to \x9f, \N for U+0085, \L, \P), so no generated file holds them raw. A viewer or diff may show NEL, U+2028 or U+2029 as a line break, so a committed file would review differently from what it holds, and YAML 1.1 readers take them as line breaks. alz's reader refuses these characters even escaped (spec §7.13), so they never round-trip; the escape is for the bytes on disk. Such text cannot come from YAML alz reads, but can come from a file-system path (allowed.yaml root and source) or a JSON string that alz adopt converts.

Cases 1 to 3 are built in the R2.8 emitter, pinned by the samples of class reference-reads-differently (field drift). Case 4 is not built yet: the R2.8 emitter writes these characters as the reference does.

Scope: build case 4 in crates/alz-core/src/render/yaml/text.rs, mark its samples as drift in crates/alz-core/tools/yaml-emit-samples.mjs and regenerate; drop the planned marker in spec §10.2; R5's output parity (TASK-55.6) marks all four cases as known reference drift. The task closes with R5's output parity. Out of scope: changing the reference (decision-30); refusing such text earlier in alz adopt or alz account allow (R5, R6).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Text holding U+007F to U+009F, U+2028 or U+2029 is written double-quoted with the escapes of spec §10.2, and its samples carry the drift field (mise run rust:check)
- [ ] #2 Long double-quoted text with a space between a line feed written as a line break and another line feed reads back as written: alz writes the space as one \ escape, a drift sample pins it, and the round-trip property covers it (mise run rust:check)
- [ ] #3 Spec §10.2 no longer marks anything as planned under TASK-73 (spec diff)
- [ ] #4 R5's output parity suite lists every reference drift case of spec §10.2 as known drift and checks alz's own bytes for each (acceptance test)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-23 architect: a fifth case found while writing spec §10.2. The reference writes a space between a line break and a line feed in long double-quoted text (J of 40 UTF-16 units or more, not a key) as \\ followed by a space, which reads back as a backslash. Shown with node and the yaml package 2.9.1: {k: 'a'.repeat(40) + '\n \nb\u0001'} with lineWidth 0 gives 'k: "aaa…\n\n  \\\\ \n\n  b\\x01"' and parses back as 'aaa…\n\\\nb\u0001' (round trip false). By reading crates/alz-core/src/render/yaml/text.rs (double_quoted: the push of a backslash after a run, then the space-before-line-feed branch) the Rust emitter reproduces it; not run. Reachable for text that rule 1 double-quotes, and for text that a block cannot hold (ends in blanks after its last line feed). Spec §10.2 marks it planned under this task. Also reachable by file names: alz build --adopt names an adopted agent after .claude/agents/<name>.md (src/adapters/claude/adopt-agent.ts), so a file name can carry case-4 characters into generated frontmatter.

Lead (2026-11-23): the emitter fixes (escaping, line-break read-back) land in R2.8 (TASK-55.3.8). Adoption (R5): alz build --adopt refuses text the reader would refuse (control, format, U+2028/U+2029 characters, invalid names) before it writes anything, with a diagnostic that names the source file and not the text, rather than writing a layer the next build refuses.
<!-- SECTION:NOTES:END -->
