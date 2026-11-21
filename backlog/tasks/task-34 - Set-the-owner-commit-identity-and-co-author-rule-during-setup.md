---
id: TASK-34
title: Set the owner commit identity and co-author rule during setup
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-21T03:32:00Z'
updated_date: '2026-11-21T03:57:00Z'
labels:
  - area:spec
  - area:security
milestone: m-0
dependencies: []
priority: high
ordinal: 5
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Commits in this repository and in agent-layer-personal were authored and committed under a stale shared identity left in the global ~/.gitconfig. Neither repository set it, so nothing noticed at setup time, and fixing it meant rewriting published history (main, and the personal layer tag v0.1.0 pinned in layer.lock). Rule from the owner: every commit is authored and committed by the owner (gautamr), and agent-made commits carry a Co-Authored-By trailer for the agent. alz must make this a standing instruction and set it up in every repository it sets up. Constraints: the invariants allow writes only inside the repository and the cache, so alz must never write ~/.gitconfig. The owner identity is layer content and must be validated before it reaches a git subprocess. Anything alz relies on in Claude Code or Codex for attribution must first be recorded in docs/target-facts.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The owner commit identity (name and email) is declared once in the personal layer; a lower layer cannot replace it with a shared or CI identity
- [ ] #2 Generated AGENTS.md always carries a standing Commits rule: author and committer are the owner, agent-made commits end with a Co-Authored-By trailer for the agent, never commit under a CI or shared identity
- [ ] #3 `alz init` and adoption set user.name and user.email in the repository local git config to the declared owner identity, and never write ~/.gitconfig
- [ ] #4 When the effective identity (`git config --show-origin user.email`) differs from the declared owner, `alz init`, adoption and `alz check` report where the wrong value comes from; `alz check` fails
- [ ] #5 The declared identity is validated (no newlines, control characters or option-like values) before it reaches a git subprocess
- [ ] #6 docs/LAYERING-SPEC.md describes the rule; acceptance tests cover a repository whose global identity is wrong, and builds stay byte-identical
- [ ] #7 Messages, tests, fixtures and docs never echo the wrong identity value they detect: reports name only its source file and scope, and fixtures use reserved domains (example.com, .invalid)
<!-- AC:END -->
