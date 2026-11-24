---
id: TASK-86
title: >-
  One collision helper with locations, and one place for output and diagnostics
  in the commands
status: To Do
assignee: []
created_date: '2026-11-24T06:34:00Z'
labels:
  - area:rust
milestone: m-7
dependencies: []
priority: low
ordinal: 157390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R5 review (2026-11-24): FileTree collisions are folded in three places (render/rendered.rs, target/claude.rs, render/passthrough.rs), two without a location; write_text is copied in seven commands beside report::write_through; init formats its diagnostics by hand and prints RepoError unstripped. Unify them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every collision names its layer and file; one output helper; init uses Diagnostic and emit_diagnostics
<!-- AC:END -->
