---
id: TASK-55.6.11
title: 'R5.11: alz doctor and alz ls --live'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
labels:
  - area:rust
  - area:cli
  - area:security
milestone: m-7
dependencies:
  - TASK-55.6.10
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 143390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§8.4, §4.4, §10, §9.1).
- alz-io tool.rs: ToolRunner for the claude binary (ALZ_CLAUDE_BIN or claude): argument arrays, no shell, stdout and stderr capped. Version: `--version`, parsed and compared with the verified range in target/claude/facts.rs. Probe: the stream-json init event, parsed in alz-core probe.rs into agent, skill and MCP server names and statuses only.
- alz-io account_names.rs: for each Claude account folder (~/.claude, ~/.claude-*, CLAUDE_CONFIG_DIR) the names in agents/ and skills/ and whether CLAUDE.md exists; never opens a file there.
- doctor.rs (pure): collisions with built names, the version line, client root and ancestor CLAUDE.md, AGENTS.md and .git, workspace folders, then the build diagnostics; exit 1 on any problem.
- ls --live: errors for built items Claude did not load, warnings for MCP status and Claude's warnings, the extra items line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz doctor reports a user-level skill or agent with a built name as a collision and a CLAUDE.md at the client root as a warning, and gives identical output under two empty account folders (tests 05, 12 and 16 against ALZ_BIN)
- [ ] #2 alz ls --live passes with the fake Claude that loads everything and fails naming the agent the other fake did not load (test 22 against ALZ_BIN)
- [ ] #3 No file inside an account folder is opened by doctor (alz-io test with the folder's files at mode 000)
<!-- AC:END -->
