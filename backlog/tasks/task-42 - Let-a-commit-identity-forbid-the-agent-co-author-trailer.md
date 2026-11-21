---
id: TASK-42
title: Let a commit identity forbid the agent co-author trailer
status: To Do
assignee: []
created_date: '2026-11-21T13:29:00Z'
labels:
  - area:spec
  - area:security
dependencies:
  - TASK-34
priority: medium
ordinal: 8390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The generated Commits section (LAYERING-SPEC §7.12, task-34) always tells agents to end their commits with a `Co-Authored-By:` trailer, and no layer can replace or remove that section. Some owners and clients do not allow any agent or tool attribution in commit history, for example when the history is a contracted deliverable read by the client's own team. Today such a user cannot adopt alz without the generated rule contradicting their own. They need a declared choice, made in the same layer that declares the identity, that keeps the author and committer rule and states that no trailer is added. The default must stay exactly as it is, so existing stacks build byte-identical output. Claude Code exposes `attribution.commit` and `attribution.pr` (target-facts §10), so a forbid choice can also be backed by the tool's own setting rather than by instruction only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A layer that may declare `commit_identity` can also declare whether agent commits require or forbid a `Co-Authored-By:` trailer; omitting it means require, and every existing fixture builds byte-identical output
- [ ] #2 With forbid, the generated Commits section keeps the author, committer and identity-check rules and states that agent-made commits carry no `Co-Authored-By:` trailer and no agent, model or session reference
- [ ] #3 The client layer's choice replaces the first layer's together with the identity it declares, under the same who-may-declare and which-wins rules as `commit_identity`
- [ ] #4 Whether alz also writes the Claude attribution setting for forbid is decided and recorded (a board decision), and any reliance on that setting is recorded in docs/target-facts.md first
- [ ] #5 docs/LAYERING-SPEC.md §7.12 describes the choice, acceptance tests cover require, forbid and the default, and the release notes name the new field
<!-- AC:END -->
