---
id: TASK-109
title: Check every adapter output in the core before the writer
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
updated_date: '2026-11-25T19:14:00Z'
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
