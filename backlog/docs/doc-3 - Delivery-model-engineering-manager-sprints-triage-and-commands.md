---
id: doc-3
title: 'Delivery model: engineering manager, sprints, triage and commands'
type: specification
created_date: '2026-11-20T16:57:00Z'
updated_date: '2026-11-20T16:57:00Z'
tags:
  - plan
  - process
  - delivery
---
# Delivery model: engineering manager, sprints, triage and commands

How work is planned, assigned and reviewed in projects built with alz, starting with alz
itself (the "Delivery kit" milestone) and piloted at the same time in a client hub. Agreed on
2026-11-20.

## The engineering manager (EM)
- One agent owns sprint planning, assignment to agents, bug triage, retros and status.
- It runs as the **main session** (`claude --agent engineering-manager`), not as a subagent:
  assigning work means spawning agents, and nested spawning has changed several times in 2026
  (allowed to depth 5, then disabled, then depth 3 by default via
  `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`). Some hubs set that variable to 1. Verify on the
  installed version before relying on nesting.
- Separation of duties: the EM plans and assigns, never codes, and never approves its own
  risky plans (`plan-approval` tasks need the owner). "Dev agents code, planning agents plan."
- Across repositories it assigns through the board (repo label + assignee); work runs in each
  repository's own session, because a hub session must not edit sibling repos.

## The kit and where it lives
| Item | Kind | Purpose |
| --- | --- | --- |
| engineering-manager | agent | leads sprints, triage, retros; assigns to agents |
| sprint-planning | skill | capacity, dependencies, plan-approval, balance across repos, sprint mechanics |
| bug-triage | skill | severity S1-S3, reproduce, label, assign, archive with a reason |
| task-routing | skill | which agent takes which label or area (moves out of /next-task) |
| retro | skill | lessons go into the narrowest layer they apply to |
| /plan-sprint, /triage, /retro, /standup, /bug, /next-task | commands | thin entry points the person triggers |

Generic items go in the owner's **personal layer**; project policy (severity examples, capacity,
cadence) goes in a **client or repo profile** the skills read. Piloted in this repository's
`.layer/` and in one client hub, then promoted with `alz promote`.

## Commands versus skills
Facts: Claude Code merged custom commands into skills; every skill is in the `/` menu with
autocomplete by default; `disable-model-invocation: true` makes a user-only command;
`user-invocable: false` makes a model-only skill. A command cannot be preloaded into an agent
(docs/target-facts.md section 5).

| What it is | Make it |
| --- | --- |
| knowledge and standards | a skill, model-invocable, preloadable |
| a ceremony or a step with side effects the person decides to run | a command (alz `commands/`) |
| a prompt typed three or more times | a thin command naming the skill and agent, with argument_hint |
| a one-off prompt | leave it as a prompt |

Never put knowledge inside a command: the EM agent could not carry it.

## Board conventions
- Statuses: Triage, To Do, In Progress, In Review, Done.
- Claim before work: set status In Progress and the assignee before starting, so two agents do
  not take the same task (Backlog.md has no locking).
- Sprints: Backlog.md has no sprint, point or velocity feature. Either milestones are sprints
  (with a due date; roadmap as epics and labels) or sprints are labels (milestones stay as the
  roadmap). Decided in the Delivery kit milestone (decision proposed).
- Bugs: type `bug`, status Triage. Severity S1 critical (money, security, data loss) goes into the
  current sprint at high priority; S2 major into the current or next sprint; S3 minor to the
  backlog; not-a-bug is archived with a note. Fix definition of done: a regression test that fails
  before and passes after, the fix, the task ID in the commit, a review.

## A developer's day
| When | Where | What |
| --- | --- | --- |
| morning | hub or repo, `claude --agent engineering-manager` | `/standup`, then `/triage` |
| working | each repo, `claude` or `claude -w <name>` | `/next-task`: implement, review, pull request |
| bug found | any session | `/bug "..."` (type bug, status Triage) |
| watching | Agent Lens, `backlog board`, `backlog browser` | who does what with which skill and how much context |
| approvals | the owner | plan-approval items, merges, releases |
| sprint end | hub | `/retro`, then `/plan-sprint` |

## Path to a "dark factory"
Dan Shapiro's five levels (January 2026) end with agents writing, reviewing and testing while
people set intent and judge outcomes; StrongDM is the most cited case; BCG: lights-out does not
mean uncontrolled. Accounts are mostly self-reported. Autonomy grows only as fast as verification.

| Stage | People | Needs |
| --- | --- | --- |
| now | drive each session | layers, agents, skills, hooks, Backlog |
| next | plan and approve; agents implement and review per task | CI, use-case traceability |
| managed | EM runs sprints, background sessions per repo, CI gate, owner approves risky items | delivery kit, Agent Lens, worktree per task, sandbox for unattended runs, budget fallback |
| dark | set intent and review outcomes | scenario-based acceptance tests, cross-model review, an orchestrator, audit trail, rollback |

## Open problems (not solved anywhere yet)
- Nested subagent spawning: changed several times in 2026; docs and changelog disagreed.
- Agent teams: experimental, off by default, no nested teams, only the lead manages.
- No native multi-repository coordinator; the board is the coordination channel.
- No task locking in Backlog.md (claim convention above); no sprints, points or velocity.
- Verifying the verifier: flaky tests, reviewers that agree with each other, no independent audits.
- `disable-model-invocation` reported to block even a typed command in VS Code (issue 26251).
- Project-level plugins do not reliably prompt to install (issue 32606).
- Prompt injection through task text, issues and community skills, worse when unattended.
- Cost and limits in unattended runs; accountability for code no person reviewed.

## Sources
- https://code.claude.com/docs/en/skills and https://github.com/anthropics/claude-code/issues/26251
- https://github.com/anthropics/claude-code/issues/19077 and https://www.digitalapplied.com/blog/claude-code-subagent-depth-limits-budget-caps-2026
- https://www.openhands.dev/blog/what-is-an-ai-dark-factory and https://env.dev/guides/dark-factory-pattern
- https://www.mintlify.com/bmad-code-org/BMAD-METHOD/reference/agents/scrum-master
