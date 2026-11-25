---
id: TASK-116
title: Specify autonomy levels, autonomy.yaml and the Claude mapping in the spec
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-25T19:07:00Z'
updated_date: '2026-11-25T19:10:00Z'
labels:
  - area:autonomy
  - needs:owner-approval
milestone: m-8
dependencies:
  - TASK-114
references:
  - docs/LAYERING-SPEC.md
priority: high
ordinal: 217390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-7 proposes the levels, the merge rule with a ceiling, the decision classes and the per-target mappings. After the owner answers doc-7 §15, the spec states them. User-facing: a new item file, new codes and new generated text and keys, so a minor version bump.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 §7.14 states the levels, the autonomy.yaml schema, the merge rule with its ceiling and the errors autonomy and autonomy-floor (grep), and §4.1 lists autonomy.yaml
- [ ] #2 §8.1 lists every Claude key a level writes with its target-facts section in brackets, and none rests on an UNVERIFIED row (grep against docs/target-facts.md)
- [ ] #3 The spec says a stack without autonomy.yaml renders exactly what it rendered before, and the status line names the minor bump (grep)
<!-- AC:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-25T19:10:00Z

---

Owner approval needed: answer doc-7 §15 (Q1-Q11) before the spec states autonomy. Recommendations are in doc-7. The ones that change the most: Q2 (a new autonomy.yaml file), Q3 (no output change without opt-in), Q4 (alz ships no classes; the personal layer ports delivery-protocol §4, which is a change outside this repository), Q7 (warn on unenforceable never rules), Q9 (a level above the ceiling is an error). Blocked: TASK-117 to TASK-120 and the autonomy part of TASK-126.

---
<!-- COMMENTS:END -->
