# Improving the layers

How the owner's layers (the personal layer and this repository's own) get better over time.
Ported from agentic-harness 1.2.2 (`SELF-IMPROVEMENT.md`), which this project replaces.

## The loop (Plan, Do, Check, Act)
1. **Observe inside.** Projects leave evidence on their boards: retrospectives (the
   retrospective skill), `area:agents` tasks, review findings, owner escalations, token reports.
2. **Observe outside.** The scout reads [SOURCES.md](SOURCES.md) weekly, updates
   [RADAR.md](RADAR.md) (Adopt, Trial, Assess, Hold) and writes a digest in `digests/`.
3. **Propose.** The steward turns radar items and project evidence into proposals: Backlog.md
   drafts here, one idea each, at most three a month.
4. **Evaluate.** A proposal that changes agent behaviour runs the eval suite (`evals/`) before
   and after. No regression in pass rate; tokens and time are compared.
5. **Decide.** The owner accepts or rejects. Lasting choices become Backlog.md decisions.
6. **Release.** Accepted changes ship as a new layer version (Semantic Versioning: patch for
   wording, minor for new capability, major for anything a project must act on).
7. **Canary, then roll out.** The new version goes to one project first; if its evidence holds
   for a week, the others follow.
8. **Prune.** Each quarter, propose removing what never fired or no longer helps.

## Roles
| Role | Writes | Cannot |
| --- | --- | --- |
| scout | `RADAR.md`, `digests/` | change layers, code, the spec or target facts |
| steward | drafts, eval results, release branches, upgrade pull requests | merge, change permissions or hooks without an accepted proposal, publish |
| owner | decisions, tags, merges | - |

## Signals per project
- Task pass rate on first review, and rework rounds.
- Blockers found in review, and blockers that escaped to the owner.
- Tokens and time per task against the project median.
- Owner escalations per week, and how many were needed.
- `area:agents` tasks by theme.

## Private parts
The steward's project list is private and lives outside this repository
(`~/.config/agent-layerz/projects.yaml`). Nothing that names a client or a private project is
written here.
