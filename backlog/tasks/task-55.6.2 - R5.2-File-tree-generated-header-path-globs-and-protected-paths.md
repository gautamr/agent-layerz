---
id: TASK-55.6.2
title: 'R5.2: File tree, generated header, path globs and protected paths'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T18:08:00Z'
updated_date: '2026-11-23T18:17:00Z'
labels:
  - area:rust
  - area:render
milestone: m-7
dependencies: []
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 134390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step, in alz-core render/.
- FileTree: BTreeMap<OutputPath, GeneratedFile>. OutputPath is a SafeRelPath with a folded key (ASCII lower case of the NFC form); inserting a path whose folded key is taken returns an error naming both paths and never replaces the first. GeneratedFile { bytes, mode: FileMode::{Regular, Executable} }. Paths iterate in code-point order (§6).
- Header: SourceRef (layer label and path), the header text of §4.3 with its legacy marker for recognising old files, html_header, the YAML comment form for frontmatter, and inserting that comment into an existing SKILL.md frontmatter, byte-identical to the reference.
- PathGlob parsed from ProtectedPattern: `**` spans folders, `*` and `?` do not, a pattern without `/` matches the file name at any depth, matched with ASCII case folding; a total matcher with no regex.
- Protected: the §4.5 defaults, the Claude memory folders (target-facts §9) and protected_paths; dropping matches from a tree with one `protected` warning each.
- lock_of: the lock's files map (sha256 of each file's bytes) and the layer entries from the ResolvedStack, through Lock::emit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Two paths that differ only by ASCII case or Unicode normalisation cannot both be in a FileTree, and the error names both (unit tests and a proptest over generated paths)
- [ ] #2 The header text, the frontmatter comment and the comment inserted into a SKILL.md match the reference's bytes for the fixtures' sources (unit tests against strings taken from the reference)
- [ ] #3 PathGlob agrees with the reference matcher on a table of patterns and paths, ignores ASCII case, and runs in time linear in pattern times path length (table tests and a proptest)
<!-- AC:END -->
