---
id: TASK-36
title: >-
  Port the agentic-harness agents and skills into the personal layer, with the
  delivery kit
status: Done
assignee:
  - '@gautamr'
created_date: '2026-11-21T09:53:00Z'
updated_date: '2026-11-21T10:13:00Z'
labels:
  - area:process
  - area:skills
  - needs:owner-approval
milestone: m-6
dependencies: []
priority: high
ordinal: 2390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The personal layer holds one skill, so repositories inherit no agents. The 12 role agents and 5 skills live only in agentic-harness/template, in native Claude format, and depend on harness files that alz-built repositories do not have (.harness/PROTOCOL.md, AUTONOMY.md, ownership.conf, guard-paths.sh). Port them to the neutral schema, turn the harness dependencies into personal skills, hooks and slots, and add the generic engineering-manager kit drafted in a client hub (task-30.3 scope) so client layers keep only overlays and policy. Owner chose the full scope on 2026-11-21. A client hub's roster task depends on this.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 agent-layer-personal/agents holds the 12 harness roles plus engineering-manager in the neutral schema: role instead of a concrete model, claude-only fields under claude:
- [x] #2 Harness runtime dependencies are replaced: the protocol and autonomy rules are a preloaded personal skill, editable paths are a slot each repository fills, the path guard is a layer hook with its script inside the layer
- [x] #3 The 5 harness skills and the delivery-kit skills (sprint-planning, bug-triage, task-routing, board conventions) are in agent-layer-personal/skills with generic content only; overlapping skills are merged, not duplicated
- [x] #4 alz lint passes on the personal layer, including the leak guard and secret scan
- [x] #5 A scratch repository stacking the unreleased layer builds, alz check passes, and alz ls --live matches
- [x] #6 The owner approves before the layer is released as v0.3.0
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Port in the personal layer working tree, verify in a scratch repository stacking it unpinned, then release only after owner approval.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Evidence 2026-11-21 (personal layer working tree, version 0.3.0, not committed or tagged):
- agents/: 12 harness roles + engineering-manager, neutral schema. Models via role (planning/review -> strong, implementation -> fast): build gives 8 opus, 5 sonnet, matching the harness. effort, maxTurns, color under claude:; isolation worktree and permission_mode plan kept.
- Harness dependencies replaced: PROTOCOL.md + AUTONOMY.defaults.md + TOKENS.md + MCP-INTAKE.md -> skill delivery-protocol (references formats, intake, tokens; slice and ADR templates in formats). PROGRESS.md -> task notes. HARNESS-FEEDBACK.md -> area:agents tasks naming the layer. Per-agent guard-paths hooks -> one layer hook role-path-guard (hooks/scripts/guard-paths.sh) keyed on the hook input's agent_type, reading ownership.conf at the repository root; no file, no agent_type or an unlisted role means allowed. Deviation from AC2 wording: ownership stays a repository file rather than an alz slot, because a strict slot would fail the build of every repository that has no ownership map.
- skills/: delivery-protocol, board-conventions, task-routing, sprint-planning, bug-triage, retrospective (was retro), record-decision. Harness next-task, new-slice, review-slice and the ceremonies became thin commands: next-task, new-slice, review-slice, plan-sprint, triage, standup, bug, retro.
- alz lint --layer . : lint ok (leak guard and secret scan).
- guard script: 9 stdin cases (no conf, main session, allowed, relative, outside area, prefix trick, .., outside repo, unlisted role) behave as intended.
- Scratch repo stacking the unpinned personal layer + a client layer + a repo overlay: alz build 43 files, 0 warnings; alz check passed; alz doctor ok; alz ls --live: Claude matches the build.
- Live hook fact: claude -p with platform-engineer limited to infra tried to write docs/probe.md; hook input carried agent_type=platform-engineer and agent_id, and the guard blocked the write. Worth recording in docs/target-facts.md.
AC6 open: release waits for owner approval.

Scope extended 2026-11-21 (owner: retire agentic-harness). Also ported to the personal layer: review-pr command; hooks hub-boundary-guard (active only in a hub, recognised by layer/layer.yaml), session-brief (session_start), notify-owner (claude:Notification via ntfy, no-op without NTFY_TOPIC); ccusage token report in retrospective; references/profiles.md (profile format, repository-instruction headings, ownership.conf). Tested: hub-guard 4 cases, notify no-op, session brief on a real board, scratch rebuild 48 files with hooks Notification 1, PreToolUse 2, SessionStart 1; alz check passed. Ported to this repository: agents scout and steward (steward's project list private, outside the repo), docs/improvement (loop, RADAR, SOURCES), evals (run.sh adapted to alz, example-001). Not ported, superseded: install-user.sh, accounts.conf, post-add-account.sh and chezmoi notes (alz writes project scope; accounts are decision-2's separate project), adopt.sh and profiles (alz init and build --adopt), doctor.sh (alz doctor), rotate-progress.sh and PROGRESS.md (task notes), tests/phase3 (installer acceptance; alz has its own), per-repo check.sh Stop hook (repository layers own it), CHANGELOG and VERSION (kept in the archive). Follow-ups: TASK-37, TASK-38, TASK-39, task-32 note.

2026-11-21: owner approved; committed 3c987c2 and tagged v0.3.0 in agent-layer-personal (author: declared noreply identity, set in that repository's local git config; earlier history uses the global address). agentic-harness deleted after verifying the archive agentic-harness-final-2026-11-21.tar.gz (file counts match); the 1.2.1 and 1.2.2 zips kept. agent-layerz now stacks v0.3.0: architect and security-reviewer became mode: replace overlays of the personal agents (+verify-target-fact, +threat-model); alz build 58 files, alz check passed, AGENTS.md 102 lines (budget 200). A client hub and its repositories stack v0.3.0 and pass alz check.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The personal layer is the single source of the role agents, the delivery kit, commands and hooks (v0.3.0); agentic-harness is retired.
<!-- SECTION:FINAL_SUMMARY:END -->
