---
id: TASK-109
title: Check every adapter output in the core before the writer
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
updated_date: '2026-11-26T03:23:00Z'
labels:
  - area:adapter
  - area:security
milestone: m-8
dependencies:
  - TASK-108
references:
  - decision-40
priority: high
ordinal: 210390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-40 point 4: the core refuses adapter output that leaves the adapter's territory, touches core-owned paths, breaks limits or uses a foreign code, and it requires a coverage report of every security item, so no adapter can weaken §4.3, §4.5 or §9. Needs a security review.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test adapter writing outside its territory, or AGENTS.md, layer.yaml, layer.lock, .layer/x or .git/x, gets the error 'target' and the build writes nothing (one test per path)
- [ ] #2 Output beyond the file-count or byte limits is the error 'target' (tests)
- [ ] #3 A code outside the shared adapter codes and the <id>- namespace is the error 'target', and a message with control characters is stripped (tests)
- [ ] #4 A deny rule or floor field missing from the coverage report fails the build; the Claude adapter reports every one as expressed (tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-26): decision-40 accepted. When an adapter cannot express a security item (for example a decision class's rules on a target without allow/ask/deny lists), alz fails the build with an error naming the item and the target, never a warning; a layer may opt that target out of the item explicitly.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-25T19:52:00Z

---

From TASK-104 (spec, @architect): §8.0 'Output checks.' and 'Coverage report.' and §10.1 'Adapter codes.' state what this task makes true, with the limits 10000 files, 16 MiB per file and 256 MiB in all, and the shared adapter codes models, tool-scope and hook-on-fail. The refusal of an item reported not expressed, and the opt-out field unenforced, are TASK-127 (owner decision of 2026-11-26), which depends on this task.

---

author: @architect
created: 2026-11-26T03:23:00Z

---

From TASK-104, after the owner's answers of 2026-11-26 (@architect): model 1.0 has four kinds of security item (spec §8.0 Model.): deny rules, floor fields that are set, the default_mode floor, and locked hooks that run on the target. The coverage check covers every item of the model, so the Claude adapter must report all four kinds as expressed, although AC #4 names only deny rules and floor fields. TASK-127 AC #4 re-checks the two new kinds.

---
<!-- COMMENTS:END -->
