---
id: TASK-108
title: Dispatch targets through an adapter registry with structured output
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
updated_date: '2026-11-26T07:14:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-107
references:
  - decision-40
priority: medium
ordinal: 209390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
render/rendered.rs matches on the target and holds CLAUDE_MANAGED_FOLDERS and CLAUDE_MEMORY_PATTERNS; CLAUDE.md is rendered in render/instructions. decision-40 points 3 and 5 move that knowledge into the Claude descriptor and adapter, and the core emits structured content and inserts the header.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 render/rendered.rs has no match on a target and no CLAUDE_ constant; adapters come from a Registry value (grep)
- [ ] #2 CLAUDE.md, the managed folders, the protected patterns and the always-adopted files come from the Claude adapter and its descriptor (grep: no claude_md in render/instructions)
- [ ] #3 The Claude adapter returns Json and Markdown content, and the core emits it and inserts the §4.3 header (test that an adapter cannot set the header)
- [ ] #4 Output is byte-identical: the acceptance suite and mise run alz:check pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-108 implementation (branch work-108). AC1: render/rendered.rs has no target match and no CLAUDE_ constant (grep CLAUDE_|match target: no hits); adapters come from target::Registry (crates/alz-core/src/target/registry.rs), built by the binary with Registry::builtin() in crates/alz/src/command/pipeline.rs and build/adopt.rs; registering refuses a model version the core cannot run and a second adapter per target. AC2: CLAUDE.md is rendered by target/claude/instructions.rs from Model.target_instructions (new field, model_for fills it from targets/claude/CLAUDE.extra.md); managed folders, protected patterns, always-adopted files and territory come from the Claude Descriptor (target/claude/descriptor.rs); render/instructions.rs has no claude_md; adopt::ADOPT_ALWAYS and protected::CLAUDE_MEMORY_PATTERNS are gone (candidates() takes the always-adopted list). AC3: the Claude adapter implements alz_adapter::Adapter and returns Json, Markdown (Mapping or Kept frontmatter + HeaderSource list) and Bytes; the core emits them in render/adapter_output (json.rs, markdown.rs) and writes every header; tests in render/adapter_output/tests.rs show an adapter cannot set the header (a forged header line stays body text after the core's header; a forged header key in the mapping stays after the core's line). AC4: mise run rust:ci green (fmt, clippy -D warnings, 1484 alz-core tests incl. fixture and golden-tree tests, doc, msrv, deny, audit); mise run alz:check: 67 generated files match. Spec gaps (spec unchanged): Model lacks the Claude-only block of CLAUDE.md, so Model.target_instructions was added (Model stays 1.0); Markdown.sources is now Vec<HeaderSource> (layer file, generated file such as AGENTS.md, slot stub) because CLAUDE.md and slot stubs name sources that are not layer files. Descriptor vocabulary stays empty until TASK-110. Known edge: a kept SKILL.md whose closing --- is the last bytes without a line feed now gains one.

Follow-up: a kept SKILL.md whose closing --- is the last bytes with no line feed is now written exactly as before. Frontmatter::Kept's body is the text after the closing --- fence byte for byte (its line feed included when the file has one), and the core writes '---' then that body with no added line feed. Test a_kept_frontmatter_without_a_final_line_feed_stays_as_it_was compares the adapter plus emitter output with the old insert_frontmatter_header result for five texts (LF and CRLF, no final line feed, empty frontmatter, body without a final line feed). mise run rust:ci green (1485 alz-core tests, doc, msrv, deny, audit); mise run alz:check: 67 generated files match.
<!-- SECTION:NOTES:END -->
