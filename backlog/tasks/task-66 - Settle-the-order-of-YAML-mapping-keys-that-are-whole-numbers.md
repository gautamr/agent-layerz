---
id: TASK-66
title: Settle the order of YAML mapping keys that are whole numbers
status: To Do
assignee: []
created_date: '2026-11-23T03:51:00Z'
updated_date: '2026-11-23T04:24:00Z'
labels:
  - discovered
  - area:spec
  - area:rust
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
  - src/core/yaml.ts
  - src/adapters/claude/agents.ts
priority: medium
type: bug
ordinal: 83390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while planning TASK-64. A JavaScript object always lists keys that are whole numbers ("0", "9", "10", not "007") first, in numeric order, whatever their insertion order. The TypeScript reference reads layer YAML into plain objects (src/core/yaml.ts) and writes frontmatter from them, so source order is not kept for such keys: a native field under an agent's or skill's claude: map that is a whole number is written before name: (src/adapters/claude/agents.ts Object.assign, then stringifyYaml), and mcp.yaml or settings.yaml entries reach merge Maps and listings (alz ls, alz explain) in that hoisted order. A Rust port that keeps source order (an ordered map) would not be byte-identical. TASK-64 fixes only sorted output (stableJson); this task decides insertion-order output. Out of scope: sorted JSON keys (TASK-64).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The spec states the key order of generated frontmatter and of mapping entries read from layer YAML, including keys that are whole numbers
- [ ] #2 The Rust alz follows it; an acceptance test with native frontmatter keys 10 and 9 after the fixed fields checks the generated agent file, marked reference drift where the TypeScript reference fails it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R5 (TASK-55.6, which depends on this task); AC #2 reworded from a TypeScript fix. The spec part (AC #1) can be written before R5.
<!-- SECTION:NOTES:END -->
