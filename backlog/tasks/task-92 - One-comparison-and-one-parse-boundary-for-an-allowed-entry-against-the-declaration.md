---
id: TASK-92
title: >-
  One comparison and one parse boundary for an allowed entry against the
  declaration
status: To Do
assignee: []
created_date: '2026-11-24T14:52:00Z'
labels:
  - area:rust
  - area:refactor
milestone: m-7
dependencies: []
priority: low
ordinal: 179390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From the R6 unit review. Whether an allowed.yaml entry still matches the cached declaration is answered in three places (resolution steps, the allow hint, account allow), and the map is printed by three map_text functions; the recorded source is a String rather than AllowedSource, so account allow has an impossible branch and the hint strips validated names again. Make one AllowEntry::matches(&DeclaredAccount) in alz-core, parse the recorded source into AllowedSource in the cache reader, keep AccountName in the hint's map, and add a proptest that the hint's Changed holds exactly when resolution skips step 4 and allow does not answer already allowed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 One alz-core function decides whether an entry matches a declaration, and resolution, the hint and allow call it (grep)
- [ ] #2 The recorded source is an AllowedSource from the cache reader on (cargo test)
- [ ] #3 A proptest shows the hint, resolution and allow agree on every entry and declaration (proptest)
<!-- AC:END -->
