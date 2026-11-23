---
id: TASK-82
title: >-
  Accept only Codex settings keys with a recorded type, and check each value's
  type
status: To Do
assignee: []
created_date: '2026-11-23T18:18:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies: []
priority: high
ordinal: 133390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Follow-up of TASK-80. Codex 0.169.0 refuses the whole config.toml (user and project) when one key has the wrong type or an unknown enum value: the command exits 1 and no other key in the file, nor the user config, applies (target-facts, Codex wrong-type section). So a wrongly typed key written by alz would silently switch off every other Codex setting, hooks and permission floors included. Same rule as TASK-79 for Claude: the Codex settings file accepts only keys whose name and type are recorded in docs/target-facts.md; each value's type is checked before alz writes it (a value of the wrong type is a settings error naming the file and the key path, no value printed; Codex itself prints the value in its error, so alz must not echo it either), and any other key is a settings error. Unknown keys are not refused by Codex, so the allowlist is alz's own rule. Record the types of the ordinary (non-security) Codex keys alz should accept through the verify-target-fact skill. One table in alz-core, beside the Claude one. The spec states the rule; the TypeScript reference accepts any key (reference drift).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every accepted Codex settings key has a recorded name and type in target-facts and a row in the table; any other key is a settings error
- [ ] #2 A value of the wrong type for an accepted key is a settings error naming the file and the key path, with no value printed; tests per type
- [ ] #3 Neutral acceptance rows through lint, and the spec states the rule with its reference drift
<!-- AC:END -->
