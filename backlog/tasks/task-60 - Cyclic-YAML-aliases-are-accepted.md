---
id: TASK-60
title: Cyclic YAML aliases are accepted
status: In Progress
assignee: []
created_date: '2026-11-22T16:27:00Z'
updated_date: '2026-11-23T06:52:00Z'
labels:
  - discovered
  - area:security
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.3.11
references:
  - src/core/yaml.ts
priority: low
type: bug
ordinal: 66390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A self-referencing alias parses into a cyclic object instead of an error. Code that walks the value afterwards can loop or overflow. Deep nesting also has no limit: the parser fails by stack overflow somewhere between 200 and 1000 levels, not deterministically. Found by the YAML spike (TASK-55.1.1) against the frozen TypeScript reference. Decide per decision-21 whether TypeScript takes a fix (it must stay a correct oracle) or the spec records the intended Rust behaviour before R2 ports parsing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cyclic aliases are rejected with an input error
- [x] #2 The spec states a nesting depth limit for untrusted YAML, enforced deterministically
- [x] #3 Acceptance tests cover a cyclic alias and a document past the depth limit
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R2 (TASK-55.3, YAML with limits and the manifest model). Its acceptance cases assert the spec and are marked reference drift (test/drift.ts) where the TypeScript reference fails them. R3 depends on this task, so it closes within R2.

Architect (2026-11-23, R2 plan in TASK-55.3): fixed in the Rust model by R2.4 (TASK-55.3.4); closed by R2.11 (TASK-55.3.11), which adds the acceptance cases (referenceDrift where the reference fails) and checks these criteria with its run. This task now depends on TASK-55.3.11.

R2.11 evidence (2026-11-23): AC1 - 34-yaml-limits 'refuses a cyclic alias' (x: &a [*a]) passes against the Rust alz: exit 1, error [yaml]; reference accepts it (referenceDrift TASK-60). AC2 - spec 7.13 'YAML limits' states at most 64 nested mappings and sequences, counted as aliases expand; enforced by the yaml::parse event pre-pass. AC3 - 34-yaml-limits: 'parses a document nested 64 levels deep' passes in both binaries, 'refuses a document nested 65 levels deep' passes against Rust (error [yaml]) and is referenceDrift TASK-60 (the reference reads it).
<!-- SECTION:NOTES:END -->
