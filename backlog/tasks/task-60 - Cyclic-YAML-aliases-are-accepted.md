---
id: TASK-60
title: Cyclic YAML aliases are accepted
status: To Do
assignee: []
created_date: '2026-11-22T16:27:00Z'
labels:
  - discovered
  - area:security
milestone: m-7
dependencies: []
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
- [ ] #1 Cyclic aliases are rejected with an input error
- [ ] #2 The spec states a nesting depth limit for untrusted YAML, enforced deterministically
- [ ] #3 Acceptance tests cover a cyclic alias and a document past the depth limit
<!-- AC:END -->
