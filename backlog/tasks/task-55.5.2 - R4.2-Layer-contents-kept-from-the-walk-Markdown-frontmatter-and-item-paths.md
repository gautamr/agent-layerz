---
id: TASK-55.5.2
title: 'R4.2: Layer contents kept from the walk, Markdown frontmatter and item paths'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
labels:
  - area:rust
  - area:resolve
milestone: m-7
dependencies: []
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 115390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step.
- alz-io: the layer walk keeps the bytes it hashed. ResolvedLayer gains LayerContents (an ordered map of SafeRelPath to bytes, at most 16 MiB per file as today) and its LayerFiles is derived from those bytes, so the merge reads exactly the content the lock check covered, with no second read.
- alz-core `markdown.rs`: MarkdownDoc { front, body }. CRLF becomes LF; a file that does not start with `---` and a line feed has an empty mapping and the whole text as body; an unclosed block and frontmatter that is not a mapping are `frontmatter` errors naming the file. The block is parsed by the YAML entry point, so the limits, the tag rule and the control-character rule of every YAML file apply (decision-28).
- alz-core `layer/contents.rs`: ItemPath, a closed enum that classifies each path of §4.1 (instruction, agent, agent overlay, skill file, command, rule, hooks, permissions, mcp, models, settings, passthrough by target, CLAUDE.extra.md, other).
Laws: the digests of LayerContents equal LayerFiles; frontmatter round trip for generated documents (proptest). Untrusted input: every layer file.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The digests of each resolved layer's LayerContents equal its LayerFiles for every fixture layer, and no file is read again after its hash was checked (alz-io integration test)
- [ ] #2 Frontmatter: CRLF is normalised, a file without an opening line has an empty mapping, an unclosed block and a non-mapping are frontmatter errors naming the file, and the YAML limits and tag rules apply inside it (unit tests and a proptest round trip)
- [ ] #3 Every item path of §4.1 maps to one ItemPath variant and any other path to ItemPath::Other (table test)
<!-- AC:END -->
