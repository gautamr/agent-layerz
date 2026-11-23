---
id: TASK-55.6.4
title: 'R5.4: Claude agents, skills, commands, slot stubs and rules'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T18:08:00Z'
labels:
  - area:rust
  - area:render
milestone: m-7
dependencies:
  - TASK-55.6.2
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 136390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.2 to §7.4, §7.10, §8.1, target-facts §1, §5, §9), in alz-core target/claude/.
- Agents: `.claude/agents/<name>.md` for agents that target Claude; frontmatter name, description, tools (neutral to native names, deduplicated, joined with `, `; a scoped tool warns `tool-scope`; an empty list is the error claude-agent), model through the routing (an id that is not a Claude model value is claude-agent), permissionMode, skills, isolation and memory as the reference writes them, then the native fields in the order NativeMap holds them (TASK-66: whole-number keys keep source order); the header as a YAML comment; the body.
- Skills: every file of the winning skill directory, SKILL.md with the header comment inserted, other files byte for byte with their executable bit. Commands: a SKILL.md with name, description, argument-hint and disable-model-invocation: true. Slot stubs: the stub text of the reference.
- Rules: `.claude/rules/<name>.md` with `paths:` frontmatter and the header.
All YAML through the R2.8 emitter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every agent, skill, command, stub and rule file of fixtures 03 to 06, 17 and 18 matches the reference's bytes (Rust tests)
- [ ] #2 A generated agent whose native fields hold the keys 10 and 9 after the fixed fields writes them in source order; the acceptance case is in the suite, marked referenceDrift TASK-66 (TASK-66 AC #2)
- [ ] #3 An empty tools list and a non-Claude model id are the error claude-agent, and a scoped tool gives the warning tool-scope, each naming the agent and its file (unit tests)
<!-- AC:END -->
