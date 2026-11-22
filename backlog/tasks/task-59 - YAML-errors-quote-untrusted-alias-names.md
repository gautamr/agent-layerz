---
id: TASK-59
title: YAML errors quote untrusted alias names
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
