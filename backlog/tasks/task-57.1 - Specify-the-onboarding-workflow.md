---
id: TASK-57.1
title: Specify the onboarding workflow
status: To Do
assignee: []
created_date: '2026-11-22T14:00:00Z'
labels:
  - area:spec
  - area:process
milestone: m-3
dependencies: []
parent_task_id: TASK-57
priority: high
ordinal: 51390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Spec before code, in LAYERING-SPEC (planned, with task IDs), plus a decision with its body filled:
- The starter layer: what it is (kind, visibility public, no commit_identity, no forbid_patterns, no personal prefixes), how it is versioned and pinned, and how a personal layer stacks on it.
- alz init paths: (1) the starter, pinned to its newest tag; (2) a new personal layer created from a source (alz layer new --from); (3) an existing layer by path: or github:/git: source. Non-interactive flags for each, and what happens without a terminal or inside an agent session.
- Trust preview: before writing anything for a remote or unfamiliar layer, show its hooks (commands), MCP servers, permissions and settings, and ask; never fetch or follow an unpinned remote default.
- Machine-local config file next to accounts.yaml: its name, default_stack, and the same file-safety rules as accounts.yaml (§7.13).
- Local overrides: syntax, that build uses the override and says so, that layer.lock never records it, and that check fails while one is active; how alz update ends it.
- Where the starter lives (the owner's account or an organisation) is an owner decision recorded here.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 LAYERING-SPEC has a planned section covering the starter layer, the three init paths with flags, the trust preview, the machine-local default stack and local overrides
- [ ] #2 A decision records the starter's home and the override semantics, with its body filled
- [ ] #3 Every new file alz reads or writes is listed in §9.1 with its safety rules
<!-- AC:END -->
