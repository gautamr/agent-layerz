---
id: TASK-76
title: Record the security-relevant Claude and Codex settings keys in target-facts
status: In Progress
assignee:
  - '@docs-writer'
created_date: '2026-11-23T16:08:00Z'
updated_date: '2026-11-23T17:55:00Z'
labels:
  - area:docs
  - area:security
milestone: m-7
dependencies: []
priority: high
ordinal: 131390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R4.16 enforces floors and reserved keys for settings keys most of which are not yet in docs/target-facts.md (the architect's draft table in TASK-55.5's notes marks them UNVERIFIED, read from Claude Code 2.1.323's settings reference and Codex 0.169.0's config schema). Verify each key's name, type and effect with the verify-target-fact skill and record it, at DOCS level at least, before R5's adapter relies on any of them; correct R4.16's list where a key turns out wrong.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every key of the approved table has a target-facts entry with its source and level
- [ ] #2 Any key that turns out wrong or renamed is fixed in the R4.16 list and the spec
<!-- AC:END -->
