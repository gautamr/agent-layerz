# Radar
Rings follow the Thoughtworks Technology Radar: Adopt (default in the harness), Trial (use on one project with evals), Assess (worth studying), Hold (don't use for now). The scout updates this weekly; the owner approves ring moves. Ported from agentic-harness 1.2.2; "Since" is the harness version that adopted the item.

Last updated: 2026-11-17

| Item | Ring | Why | Since |
| --- | --- | --- | --- |
| AGENTS.md as the shared instruction file (Agentic AI Foundation) | Adopt | One file read by Codex and, via import, Claude Code | 1.0.0 |
| Agent Skills (SKILL.md) for procedures | Adopt | Open format read by 30+ tools; loads only when used | 1.0.0 |
| MCP with deferred tool loading | Adopt | Standard tool protocol; tool search keeps context small | 1.0.0 |
| Role subagents with file ownership | Adopt | Fresh context per role; hooks enforce boundaries | 1.0.0 |
| Spec, plan, tasks, tests, then code | Adopt | Clear definition of done for every agent | 1.0.0 |
| Hooks for rules with no exceptions | Adopt | Deterministic, unlike instructions | 1.0.0 |
| Backlog.md as the task queue | Adopt | Markdown in git, MCP for Claude and Codex, board for the owner | 1.0.0 |
| ccusage for token reports | Adopt | Local, open source, covers Claude Code and Codex | 1.0.0 |
| OpenSSF Scorecard in intake | Adopt | Objective health signal for dependencies | 1.0.0 |
| Ralph Loop plugin for unattended batches | Trial | Works for well-specified tasks with checks; always cap iterations | 1.0.0 |
| Agent teams for multi-lens review | Trial | Parallel reviewers; token cost grows per teammate | 1.0.0 |
| OpenTelemetry export of Claude Code metrics | Trial | Token, cost and session metrics into any OTel backend | 1.0.0 |
| Private plugin marketplace to distribute the harness | Trial | Projects receive updates through Claude Code's plugin system | 1.0.0 |
| Groma.md live C4 architecture map | Assess | Useful architecture context for agents, especially in existing codebases; early prototype with experimental scanners | 1.1.0 |
| Beads issue tracker | Assess | Strong for large parallel fleets; stores tasks in a database | 1.0.0 |
| Claude Code built-in Tasks as shared list | Assess | Persists across sessions, but Claude-only and outside the repo | 1.0.0 |
| Cross-model review (a second model family) | Assess | Fewer shared blind spots on high-risk diffs | 1.0.0 |
| Many always-on MCP servers | Hold | Context bloat and a larger attack surface | 1.0.0 |
| Auto-merging agent pull requests | Hold | Removes the owner's final control | 1.0.0 |
| Agents editing their own permissions or hooks | Hold | Defeats enforcement | 1.0.0 |
