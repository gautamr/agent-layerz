---
id: TASK-127
title: Refuse unexpressed security items unless a layer opts the target out
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-25T19:30:00Z'
labels:
  - area:adapter
  - discovered
milestone: m-8
dependencies:
  - TASK-109
references:
  - decision-40
  - docs/LAYERING-SPEC.md
priority: high
type: feature
ordinal: 228390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner decision on 2026-11-26 (TASK-104 notes): when an adapter cannot express a security item, alz fails the build with an error naming the item and the target, never a warning, and a layer may opt that target out of the item explicitly. TASK-109 adds the coverage report and refuses a report that leaves an item out; this task adds the refusal of an item reported as not expressed and the opt-out field unenforced of permissions.yaml, as spec §7.6 'Targets that cannot enforce an item' and §8.0 'Security items a target cannot enforce' state (TASK-104). User-facing: a new layer field, so a minor version bump and a CHANGELOG entry. Out of scope: autonomy items (doc-7) and the Codex adapter (TASK-126), which is the first adapter that needs it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 permissions.yaml accepts unenforced.<target id>.deny and .floor_fields as spec §7.6 says; another shape, a rule this layer does not deny or a floor field it does not set to true is the error 'permissions', also in alz lint (tests)
- [ ] #2 A test adapter that reports a deny rule or a floor field as not expressed fails the build with the error 'target' at <layer>:permissions.yaml and the message of spec §8.0 naming the item and the target, and nothing is written (test)
- [ ] #3 When every layer that declares the item opts the target out of it the build passes with no diagnostic for it; an entry of a more specific layer never covers a more general layer's item (tests)
- [ ] #4 docs/spec-coverage.md rows §7.6 Targets that cannot enforce an item and §8.0 Security items a target cannot enforce are covered; the acceptance suite, mise run rust:check and mise run alz:check pass with unchanged output
<!-- AC:END -->
