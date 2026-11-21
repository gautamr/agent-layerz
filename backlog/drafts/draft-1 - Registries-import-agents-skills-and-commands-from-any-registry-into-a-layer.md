---
id: DRAFT-1
title: 'Registries: import agents, skills and commands from any registry into a layer'
status: Draft
assignee:
  - '@architect'
created_date: '2026-11-19T18:14:00Z'
labels:
  - idea
  - area:registry
  - area:spec
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner's idea (2026-11-19): developers should be able to import agents, skills and commands from any registry and have alz integrate them instantly, with the owner running their own registry, and community and custom items combined in one stack.

Starting points:
- A layer is already a registry unit: github:/git: sources pin a whole layer. The gap is item-level import (one agent or skill), discovery, versions and trust.
- Interoperate before inventing a format: Claude plugin marketplaces (.claude-plugin/marketplace.json), Agent Skills folders (SKILL.md), and registry CLIs such as 'npx skills add'. Verify each format before designing.
- Possible shape: registries declared in a layer; 'alz add <type> <name> --from <registry>@<version>' either vendors the item into a layer (with license and source recorded) or references it pinned; layer.lock records the content hash; the leak guard and threat model apply to every import; conversion to the neutral schema reuses --adopt.
- Precedence follows decision-6: custom items decide what and where, community items guide how.

Open questions: vendor or reference by default; how updates and removals work; licence handling; how the owner's own registry is published and authenticated.
<!-- SECTION:DESCRIPTION:END -->
