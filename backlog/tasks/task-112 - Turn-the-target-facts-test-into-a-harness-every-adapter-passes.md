---
id: TASK-112
title: Turn the target-facts test into a harness every adapter passes
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-111
references:
  - decision-40
  - docs/target-facts.md
priority: medium
ordinal: 213390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/alz-core/src/target/claude/tests/facts.rs enforces spec §8 'rely only on recorded facts' for Claude with Claude-specific extractors. decision-40 point 7 generalises it: structured output lets one harness list the written keys of any adapter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The harness lists keys from structured output and checks them against an adapter's facts file and subject map: a fake adapter with an unrecorded key fails, a recorded key passes, a stale NO_FACT_RECORDS entry fails (tests)
- [ ] #2 The Claude facts test uses the harness, with its subject map and NO_FACT_RECORDS unchanged in content (diff)
- [ ] #3 Each descriptor names its facts file, and spec §8 says the rule applies to every adapter (grep)
<!-- AC:END -->
