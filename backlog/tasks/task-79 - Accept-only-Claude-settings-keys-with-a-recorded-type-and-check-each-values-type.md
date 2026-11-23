---
id: TASK-79
title: >-
  Accept only Claude settings keys with a recorded type, and check each value's
  type
status: To Do
assignee: []
created_date: '2026-11-23T18:09:00Z'
labels:
  - area:rust
  - area:security
  - area:merge
milestone: m-7
dependencies: []
priority: high
ordinal: 149390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner decision (2026-11-23): settings.yaml under claude: accepts only keys whose name and type are recorded in docs/target-facts.md; each value's type is checked before alz writes it (a value of the wrong type is a settings error naming the file and the key path, no value printed), and any other key is a settings error. Start from TASK-76's §13 of target-facts and record the types of the ordinary (non-security) Claude settings keys alz should accept, through the verify-target-fact skill. Agents' native fields under claude: get the same check for the frontmatter keys Claude reads. One table in alz-core (beside item/native_keys.rs). The spec states the rule; the TypeScript reference accepts any key (reference drift).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every accepted Claude settings key has a recorded name and type in target-facts and a row in the table; any other key in settings.yaml is a settings error
- [ ] #2 A value of the wrong type for an accepted key is a settings error naming the file and the key path, with no value printed; tests per type
- [ ] #3 Neutral acceptance rows through lint, and the spec states the rule with its reference drift
<!-- AC:END -->
