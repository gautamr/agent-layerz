---
id: TASK-64
title: Sort names by Unicode code point everywhere, as the spec says
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-22T19:20:00Z'
updated_date: '2026-11-23T03:44:00Z'
labels:
  - discovered
  - area:spec
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
  - src/core/files.ts
priority: high
type: bug
ordinal: 80390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found in R1.2 (TASK-55.2.2). Spec §5.3 says code point order, but src/core/files.ts compareCodePoints uses `<` (UTF-16 code unit order) and many sites use plain .sort(); the content hash, the generated file tree (src/render/file-tree.ts) and account state follow it. Names with characters above U+FFFF sort differently from the spec, so a Rust port that sorts by code point would not be byte-identical. Owner decision (2026-11-23): keep code point order and fix the TypeScript reference (a fix both carry, decision-21). test/reference/spec-formulas.test.ts has an it.fails KNOWN DRIFT case for this.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 One code point comparison is used at every place src/ orders names or keys that reach generated output, the lock or a hash
- [ ] #2 An acceptance test with a name above U+FFFF and one near U+FF61 proves the order, and the KNOWN DRIFT case in spec-formulas.test.ts becomes a normal passing case
- [ ] #3 The spec states the order once and the changelog notes that layers with such names get a new content hash
<!-- AC:END -->
