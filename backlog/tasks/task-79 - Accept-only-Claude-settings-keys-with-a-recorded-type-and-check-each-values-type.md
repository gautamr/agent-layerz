---
id: TASK-79
title: >-
  Accept only Claude settings keys with a recorded type, and check each value's
  type
status: Done
assignee: []
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T03:14:00Z'
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
- [x] #1 Every accepted Claude settings key has a recorded name and type in target-facts and a row in the table; any other key in settings.yaml is a settings error
- [x] #2 A value of the wrong type for an accepted key is a settings error naming the file and the key path, with no value printed; tests per type
- [x] #3 Neutral acceptance rows through lint, and the spec states the rule with its reference drift
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Done in PR #12 (merged 2026-11-24).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
settings.yaml under claude: and agents' claude: fields accept only keys whose name and type are recorded in target-facts §14 (147 settings rows, 10 agent fields); a value of the wrong type or another key is an error naming the file and key path without the value; three more keys reserved. Merged as PR #12.
<!-- SECTION:FINAL_SUMMARY:END -->
