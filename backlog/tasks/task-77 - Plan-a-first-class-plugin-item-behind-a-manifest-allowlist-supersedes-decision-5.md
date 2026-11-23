---
id: TASK-77
title: >-
  Plan a first-class plugin item behind a manifest allowlist (supersedes
  decision-5)
status: To Do
assignee: []
created_date: '2026-11-23T16:08:00Z'
labels:
  - area:spec
  - area:security
milestone: m-7
dependencies: []
priority: medium
ordinal: 132390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner (2026-11-23): claude.enabledPlugins and extraKnownMarketplaces are reserved in settings.yaml because plugins can bring hooks and MCP servers past the mcp.enable allowlist and hooks.yaml. Decision-5's title declares community skills as plugins in settings.yaml enabledPlugins. Record a decision superseding decision-5 and plan a first-class plugin item: which layers may declare plugins, a manifest allowlist like mcp.enable, how plugin hooks and servers meet the lock and allowlist rules, and the Codex counterpart.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A decision superseding decision-5 is recorded and linked
- [ ] #2 A plan with subtasks for the plugin item exists, after the Rust cutover or as the owner schedules
<!-- AC:END -->
