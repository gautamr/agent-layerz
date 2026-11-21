---
id: doc-2
title: Agent Lens plan
type: specification
created_date: '2026-11-19T17:59:00Z'
updated_date: '2026-11-19T17:59:00Z'
tags:
  - plan
  - agent-lens
---
# Agent Lens plan

A Claude Code mod that shows, live and beside the chat, which agent, skill, memory and how
many tokens Claude is working with, and which alz layer each item comes from. It is built by
this project's own agent team, combining custom and community skills, and tracked as the
"Agent Lens" milestone.

## Use cases
Each task and test traces back to one of these IDs.

| ID | As a person running Claude Code, I want to… | So that… |
| --- | --- | --- |
| UC-L1 | see which subagent is working, on which task, with which model and for how long | I know who is doing what in a multi-agent run |
| UC-L2 | see which skills are active, how each was loaded (slash command, Skill tool, preload) and from which layer or plugin | I can tell whether the right guidance is in play |
| UC-L3 | see which memory and instruction files and rules are loaded, at which level (User, Project, Local, Managed) and why (session start, path match, include, compact) | I can tell which rules the agent is following |
| UC-L4 | see context use, cost and rate-limit headroom, with a warning before a limit or a compaction | I am not surprised by a full context or a limit |
| UC-L5 | see recent tool calls, failures and permission prompts | I can follow the work without reading the whole transcript |
| UC-L6 | get a compact view in a narrow terminal | it is useful below 144 columns too |
| UC-L7 | install it with one line, and know nothing leaves my machine | I can trust and adopt it |

## Team
| Agent | Role |
| --- | --- |
| architect | verifies what the mod API observes, writes the design, splits the work into tasks that touch separate files |
| tui-designer (new) | owns the pane's layout and behaviour at 80, 110 and 144+ columns, truncation, colour plus symbols, refresh budget |
| implementer x2-3 | builds in parallel, each in its own git worktree (`isolation: worktree`) |
| reviewer, security-reviewer | fresh-context reviews; security matters because the mod reads files and sees all session activity |
| release-engineer | packaging as an installable plugin; `claude plugin validate` and `claude plugin test` in CI |
| docs-writer | README section, screenshots, install instructions |

The main session runs the waves; `/next-task` handles single tasks in board order.

## Skills: custom and community
| Source | Skills | Used for |
| --- | --- | --- |
| Community: obra/superpowers (official Claude plugin marketplace) | brainstorming | the design session |
| | writing-plans, executing-plans | plans, recorded in the Backlog task |
| | test-driven-development | test first, with this repo's test conventions |
| | subagent-driven-development, dispatching-parallel-agents | running the build wave in parallel |
| | systematic-debugging, verification-before-completion | fixing failures; proof before claiming done |
| | writing-skills | writing the new custom skills |
| Built-in (Anthropic) | plugin-authoring | the mod API: panes, events, state, tests |
| Custom, existing | verify-target-fact, threat-model, add-acceptance-test | proving events fire; security review; the alz side |
| Custom, new | claude-code-mod | this repo's mod conventions: folder, validate and test commands, state contract, verified version range |
| | tui-design | terminal layout rules for panes |

Community skills arrive through alz: superpowers is declared in `.layer/settings.yaml`
(`enabledPlugins`), which alz writes into `.claude/settings.json`. Caveats to check in L1: a
reported Claude Code issue says project-level plugins are not always offered for install (fall
back to a documented `/plugin install` line), and the plugin's context cost is measured with
`/context` before and after; the owner decides whether to keep it.

**Precedence:** repo instructions and custom skills decide *what* and *where*; community
skills guide *how*. Example: writing-plans normally writes plan files; here the plan goes into
the Backlog task.

## Waves
1. Setup: L1, then L2. L11 can run any time.
2. Research: L3, then the L4 design. The owner reviews the design before any code.
3. Build: L5 and L6 in parallel worktrees, then L7.
4. Ship: L8, L9, then the L10 review.

## Definition of done (epic)
- In a session in this repo, the pane shows the active subagent, skills, loaded memory files
  with their layer, context use, cost and recent tools, live (UC-L1 to UC-L5).
- It degrades to a compact view below 144 columns (UC-L6).
- Plugin tests pass in CI; the security review has no open findings.
- Installing takes one `/plugin install` line (UC-L7).

## Risks
- The mod API is new (Claude Code 2.1.311) and may change: the claude-code-mod skill pins the
  verified version range, as alz adapters do.
- Community skills may conflict with project rules: the precedence rule, and L1 measures cost.
- Narrow terminals: an unasked pane opens only at 144+ columns, so the design needs a compact mode.

## Sources
- https://code.claude.com/docs/en/plugins/org (project-scope plugins: enabledPlugins, extraKnownMarketplaces)
- https://github.com/anthropics/claude-code/issues/32606 and /issues/9537 (install prompts not reliable)
- https://claudemarketplaces.com/skills/obra/superpowers/brainstorming (superpowers skills)
