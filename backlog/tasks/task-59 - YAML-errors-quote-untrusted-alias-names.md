---
id: TASK-59
title: YAML errors quote untrusted alias names
status: Done
assignee: []
created_date: '2026-11-22T16:27:00Z'
updated_date: '2026-11-23T07:29:00Z'
labels:
  - discovered
  - area:security
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.3.11
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
- [x] #1 Errors from parsing untrusted YAML never contain text from the input; they name the file and the kind of error (and a line number if available)
- [x] #2 An acceptance test with a marker string in an unresolved alias proves the marker is not printed
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R2 (TASK-55.3, YAML with limits and the manifest model). Its acceptance cases assert the spec and are marked reference drift (test/drift.ts) where the TypeScript reference fails them. R3 depends on this task, so it closes within R2.

Architect (2026-11-23, R2 plan in TASK-55.3): fixed in the Rust model by R2.4 (TASK-55.3.4); closed by R2.11 (TASK-55.3.11), which adds the acceptance cases (referenceDrift where the reference fails) and checks these criteria with its run. This task now depends on TASK-55.3.11.

R2.11 evidence (2026-11-23, branch feat/rust-r2-task-55.3): AC1 - test/acceptance/34-yaml-limits.test.ts 'refuses an unresolved alias and never prints its name' passes against ALZ_BIN=target/release/alz: exit 1, error [yaml], the marker is in neither stdout nor stderr; the Rust YAML errors name file, line, column and kind only (spec 7.13 'What else makes a YAML file invalid', R2.4 unit and property tests). AC2 - that same case is the marker test; against the bundled reference it fails (the reference prints 'Unresolved alias ...: secret-marker-9f3a1c'), so it is referenceDrift('TASK-59'), an expected failure with ALZ_BIN unset. Status stays In Progress until the lead moves it after the unit review.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed in the Rust alz in R2 (PR #7, decision-30); the TypeScript reference keeps the bug, and its acceptance cases are marked reference drift.
<!-- SECTION:FINAL_SUMMARY:END -->
