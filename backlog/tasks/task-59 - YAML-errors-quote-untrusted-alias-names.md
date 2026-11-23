---
id: TASK-59
title: YAML errors quote untrusted alias names
status: To Do
assignee: []
created_date: '2026-11-22T16:27:00Z'
updated_date: '2026-11-23T04:24:00Z'
labels:
  - discovered
  - area:security
  - area:rust
milestone: m-7
dependencies: []
references:
  - src/core/yaml.ts
priority: medium
type: bug
ordinal: 65390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
parseYaml and parseYamlLimited put the first line of the yaml library's message into the InputError. For an unresolved alias that line contains the alias name taken from the untrusted file, so repository content reaches the terminal (a secret-looking marker showed up in the spike's output). Found by the YAML spike (TASK-55.1.1) against the frozen TypeScript reference. Decide per decision-21 whether TypeScript takes a fix (it must stay a correct oracle) or the spec records the intended Rust behaviour before R2 ports parsing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Errors from parsing untrusted YAML never contain text from the input; they name the file and the kind of error (and a line number if available)
- [ ] #2 An acceptance test with a marker string in an unresolved alias proves the marker is not printed
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R2 (TASK-55.3, YAML with limits and the manifest model). Its acceptance cases assert the spec and are marked reference drift (test/drift.ts) where the TypeScript reference fails them. R3 depends on this task, so it closes within R2.
<!-- SECTION:NOTES:END -->
