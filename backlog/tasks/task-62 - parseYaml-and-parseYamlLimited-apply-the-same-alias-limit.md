---
id: TASK-62
title: parseYaml and parseYamlLimited apply the same alias limit
status: To Do
assignee: []
created_date: '2026-11-22T16:27:00Z'
labels:
  - discovered
  - area:spec
milestone: m-7
dependencies: []
references:
  - src/core/yaml.ts
priority: low
type: bug
ordinal: 68390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The yaml library's default maxAliasCount is 100, the same as MAX_ALIASES, so parseYamlLimited behaves exactly like parseYaml. The spec reads as if only some files get the alias limit; in fact all do. Neither caps input size: only some callers check 1 MiB first. Found by the YAML spike (TASK-55.1.1) against the frozen TypeScript reference. Decide per decision-21 whether TypeScript takes a fix (it must stay a correct oracle) or the spec records the intended Rust behaviour before R2 ports parsing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The spec states one parsing rule for all YAML alz reads (alias limit, size cap, depth) or names which files differ and why
- [ ] #2 The Rust port's single YAML entry point follows that rule
<!-- AC:END -->
