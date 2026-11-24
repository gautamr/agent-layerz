---
id: TASK-55.6.13
title: 'R5.13: Adoption planning'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T03:42:00Z'
labels:
  - area:rust
  - area:adopt
  - area:security
milestone: m-7
dependencies:
  - TASK-55.6.6
  - TASK-55.6.1
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 145390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§4.3), pure in alz-core adopt/.
- Inputs: the collisions and the adopt-always files (.claude/settings.json, .mcp.json) that alz did not generate, read by RepoRoot as bytes; the Merged stack before adoption; the repo layer's existing files.
- AdoptionPlan: layer files to write (YAML through the R2.8 emitter, merged into existing layer YAML with maps deep and lists appended), moves, removals, manifest additions (workspace.add_dirs, mcp.enable), the report, notes and problems. AGENTS.md to instructions/00-adopted.md; CLAUDE.md to targets/claude/CLAUDE.extra.md without @AGENTS.md and without paragraphs AGENTS.md holds; agents to neutral agents (native-only fields under claude:, models as aliases, an agent a general layer defines as a mode: replace overlay); skills and rules moved; hook scripts to hooks/; settings.json to permissions.yaml, hooks/hooks.yaml, settings.yaml, the models default and add_dirs; .mcp.json to mcp.yaml and mcp.enable.
- TASK-73: the planned repo layer is read in memory with the R4 per-layer readers before the plan is accepted; any text the reader would refuse is a problem naming the source file, never the text.
- JSON is read with the exactness owner question 4 decides. Problems print untrusted text stripped (class 1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Adopting fixture 20 plans the layer files, moves and manifest additions the reference makes, byte for byte for the layer files (unit tests)
- [ ] #2 A literal env value in .mcp.json, an unknown permission tool, a hook type other than command, or text the layer reader would refuse is a problem, and the plan is then rejected whole (unit tests, TASK-73)
- [ ] #3 Building the adopted layer gives the settings, hooks and agents of the native files it adopted (property over generated native settings, test 20's law)
<!-- AC:END -->
