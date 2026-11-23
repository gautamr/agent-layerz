---
id: TASK-55.5.2
title: 'R4.2: Layer contents kept from the walk, Markdown frontmatter and item paths'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T12:45:00Z'
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
- [x] #1 The digests of each resolved layer's LayerContents equal its LayerFiles for every fixture layer, and no file is read again after its hash was checked (alz-io integration test)
- [x] #2 Frontmatter: CRLF is normalised, a file without an opening line has an empty mapping, an unclosed block and a non-mapping are frontmatter errors naming the file, and the YAML limits and tag rules apply inside it (unit tests and a proptest round trip)
- [x] #3 Every item path of §4.1 maps to one ItemPath variant and any other path to ItemPath::Other (table test)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (2026-11-23), R4.2 code done; ACs not checked, not committed.
Changes: alz-core layer/contents.rs (LayerContents: BTreeMap<SafeRelPath, bytes>, files() = LayerFiles digests, drops un-hashed paths like LayerFiles, Debug shows sizes only); layer/item_path.rs (ItemPath<'a> enum); markdown.rs (MarkdownDoc{front: Frontmatter, body: Body}, MarkdownError, into_diagnostic); Code::Frontmatter (frontmatter); LayerScan::with_file(path, bytes) keeps the bytes the walk read; ReadLayer::contents()/into_parts now give LayerContents; ResolvedLayer::new takes LayerContents and derives files(); alz-io FetchedLayer.files became .contents (callers use .contents.files()); resolve/stack.rs read_layer parses layer.yaml from the walked bytes (no second read; the by-name read remains only to tell a missing file from a link/folder, its bytes are never used).
AC1: alz-io/tests/layer_contents.rs walks every fixture layer (>=110 of 119 layer.yaml folders pass the walk) and compares contents.files() and the ContentHash with an independent std::fs read, and the bytes with the files; alz-core stack/resolved/tests.rs: ResolvedLayer.files() == contents.files(); layer/contents/tests.rs proptest digests == LayerFiles::from_contents. No second read: structural (ResolvedLayer holds no Dir; read_layer takes layer.yaml from contents); R3 stack_lint cases for a linked/missing layer.yaml still pass.
AC2: alz-core markdown/tests.rs: CRLF, no opening line (incl. '---' with no LF, '--- ', '----'), empty/comment/null block = empty mapping, unclosed (NotClosed, frontmatter code, names file), non-mapping (frontmatter code at file line:col), YAML errors keep yaml/yaml-tag codes with file-absolute positions (block parsed after a leading LF), alias and depth budgets, 1 MiB counts the whole file (at cap ok, +1 TooLarge; no cap without a block), control char / invalid UTF-8; proptest: generated documents (render::yaml emit) read back as the same mapping and body, also with CRLF.
AC3: alz-core layer/item_path/tests.rs: table of every §4.1 path, a table of Other paths, proptest.
Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, msrv, deny, audit); mise run lint exit 0.
Spec gaps for the architect (R4.1): (1) the frontmatter rules are not in the spec: opening line is '---' + LF (CRLF counts as LF), the block closes at the first line that is exactly '---' (the reference closes at any line starting with '---', a looser rule; '----' or '--- ' do not close here), text after the closing line (after its LF) is the body, an empty/comment-only/null block is an empty mapping, a file without an opening line is body only; (2) §10.1 has no code 'frontmatter' (used for not closed, not a mapping, and a body-only file that is not UTF-8), messages 'frontmatter is not closed with ''---''' and 'frontmatter must be a YAML mapping'; (3) a file without a block that is not valid UTF-8 is refused (reference: lossy); no 1 MiB cap and no control-character rule apply to a file without a block; (4) YAML errors inside a block use codes yaml/yaml-tag at the line and column of the file; (5) ItemPath: targets/<name>/... for a name that is not a known target is Other (the reference merges any target name, R4.12 decides); skills/<name>/<file> is one SkillFile variant for every file below a skill folder; hooks/scripts/... is HookScript; 'agents/.overlay.md' is Other.

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (689 tests), lint passes, src/ unchanged. The frontmatter rules, the frontmatter code and the unknown-passthrough-target question went to R4.1 (spec).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The layer walk keeps the bytes it hashed as LayerContents, so the merge reads exactly what the lock check covered with no second read; Markdown frontmatter is parsed through the one YAML entry point with the file's 1 MiB cap and limits; every §4.1 path is classified by a closed ItemPath enum.
<!-- SECTION:FINAL_SUMMARY:END -->
