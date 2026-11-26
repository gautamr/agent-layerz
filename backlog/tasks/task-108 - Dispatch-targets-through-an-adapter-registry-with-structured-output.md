---
id: TASK-108
title: Dispatch targets through an adapter registry with structured output
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
updated_date: '2026-11-26T05:57:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-107
references:
  - decision-40
priority: medium
ordinal: 209390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
render/rendered.rs matches on the target and holds CLAUDE_MANAGED_FOLDERS and CLAUDE_MEMORY_PATTERNS; CLAUDE.md is rendered in render/instructions. decision-40 points 3 and 5 move that knowledge into the Claude descriptor and adapter, and the core emits structured content and inserts the header.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 render/rendered.rs has no match on a target and no CLAUDE_ constant; adapters come from a Registry value (grep)
- [ ] #2 CLAUDE.md, the managed folders, the protected patterns and the always-adopted files come from the Claude adapter and its descriptor (grep: no claude_md in render/instructions)
- [ ] #3 The Claude adapter returns Json and Markdown content, and the core emits it and inserts the §4.3 header (test that an adapter cannot set the header)
- [ ] #4 Output is byte-identical: the acceptance suite and mise run alz:check pass
<!-- AC:END -->
