---
id: TASK-88
title: Recognise the generated mark only where a header sits
status: To Do
assignee: []
created_date: '2026-11-24T06:35:00Z'
labels:
  - area:rust
  - area:security
  - area:spec
milestone: m-7
dependencies: []
priority: medium
ordinal: 159390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R5 review (2026-11-24): a file counts as generated when the mark appears anywhere in its first 1 KiB (render/header.rs), so a hand-written file that quotes the mark counts as alz's own. Specify and implement that the mark counts only where alz writes the header (first line, or the line after the frontmatter opening), with reference drift if the reference differs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The spec states where the mark counts; a file quoting it elsewhere is not alz's own, with tests
<!-- AC:END -->
