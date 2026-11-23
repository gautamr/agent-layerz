---
id: TASK-65
title: Write down what the spec leaves implicit about hashes, limits and environment
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-22T19:20:00Z'
updated_date: '2026-11-23T03:24:00Z'
labels:
  - area:spec
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
priority: high
ordinal: 81390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found in R1.2 (TASK-55.2.2): the test-side formulas had to read src/ for five things the spec does not say. Clarify the current behaviour in LAYERING-SPEC with no behaviour change, before R2 ports the model. (1) §5.3 content hash byte layout: per file in order, path, NUL, sha256 hex of the bytes, LF, hashed as one text; the sha256: prefix in layer.lock; '.env files' means names starting with .env. (2) §7.13 the YAML alias limit's number (100), and that decision-28's limits supersede it for the Rust port where they differ. (3) §7.13 the full list of environment variables resolution reads. (4) Whether the 2000-entry limit counts links and skipped names. (5) XDG_CACHE_HOME empty or relative for the build cache is TASK-53's, not this task's.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each of points 1 to 4 is stated in LAYERING-SPEC, matching the TypeScript reference
- [ ] #2 The test-side formulas in test/spec/ cite those sections, and their notes no longer say the spec is silent
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From the R1 review (2026-11-23): the spec names no diagnostic (code, wording) for a failed fetch of a remote source; acceptance 26 can assert only 'error [source]' plus the source named. Specify it.
<!-- SECTION:NOTES:END -->
