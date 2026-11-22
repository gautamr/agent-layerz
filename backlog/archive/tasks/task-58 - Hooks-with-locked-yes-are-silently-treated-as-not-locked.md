---
id: TASK-58
title: 'Hooks with locked: yes are silently treated as not locked'
status: To Do
assignee: []
created_date: '2026-11-22T16:27:00Z'
updated_date: '2026-11-22T16:27:00Z'
labels:
  - discovered
  - area:security
  - area:spec
milestone: m-7
dependencies: []
references:
  - src/core/yaml.ts
  - docs/LAYERING-SPEC.md
priority: medium
type: bug
ordinal: 64390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The YAML parser follows YAML 1.2, so `locked: yes` in a hook is the string "yes", not true, and the hook is not locked. A general layer that writes yes meaning true loses the protection §7 gives locked hooks, and nothing warns. Found by the YAML spike (TASK-55.1.1) against the frozen TypeScript reference. Decide per decision-21 whether TypeScript takes a fix (it must stay a correct oracle) or the spec records the intended Rust behaviour before R2 ports parsing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Lint or build rejects a non-boolean locked value with a message naming the hook and file, instead of treating it as false
- [ ] #2 An acceptance test covers locked: yes
- [ ] #3 The spec states that locked takes only true or false
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Not a bug: src/merge/hooks.ts:178 rejects a non-boolean locked value ('locked' must be true or false), so locked: yes is an error, not a silent false. Filed from an unverified spike claim and archived the same session.
<!-- SECTION:NOTES:END -->
