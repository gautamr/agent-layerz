---
id: doc-1
title: Publication plan
type: specification
created_date: '2026-11-19T17:10:00Z'
updated_date: '2026-11-21T18:19:00Z'
tags:
  - plan
  - roadmap
---
# Publication plan

How agent-layerz becomes a well-run open source project. The work is tracked on this
board: one milestone per phase, and this project is built with `alz` itself, so every step
also tests the tool.

## Starting point (2026-11-19)
- `agent-layerz@0.1.0` is on npm with README, LICENSE and keywords. npm's staged publishing
  left a placeholder version, `0.0.0-stage`.
- `main` is one clean commit; 50 tests pass; four security issues with untrusted repo content
  were fixed before release (see the threat-model skill).
- Missing: the GitHub repository (the npm page already links to it), CI, a release process,
  contributor and security documents, and the Codex adapter promised by the spec.

## Phases (milestones)
1. **Phase 0: Make the project real.** GitHub repository with only `main`, settings, branch
   protection, private vulnerability reporting, the npm placeholder deprecated.
2. **Phase 1: Project basics.** CI on Node 22 and 24, Ubuntu and macOS; SECURITY.md with the
   threat model; CONTRIBUTING.md; CHANGELOG.md; Code of Conduct; issue and PR templates; a
   runnable example; target facts cleaned of machine paths; the personal layer public and
   used through `github:` so CI can run `alz check` on this repository.
3. **Phase 2: Automated, verifiable releases.** release-please, npm trusted publishing with
   provenance, staged releases approved by a person with 2FA, a written 0.x versioning policy.
4. **Phase 3: Roadmap to 1.0.** Codex adapter, `alz promote`, `alz update`, `alz publish`,
   optional account switching in alz with its account and trust checks in `alz doctor`
   (decision-15, TASK-50), a decision on Windows.
5. **Phase 4: Launch.** An anonymised case study from a real multi-repo client migration, a
   launch post with an honest comparison, a short terminal recording, announcements.

## Decisions
- **This repository is built with alz.** Its agents, skills, rules, hooks, permissions and MCP
  servers live in `.layer/` on top of the owner's public personal layer. Gaps found while
  doing so become tasks here.
- **alz optionally chooses each repository's account folder (decision-15, replaces
  decision-2).** Developers adopt one tool more readily than two, and alz already knows which
  client a repository belongs to. A client layer names the account its repositories expect, a
  machine-local user config can override it, and a layer-named account applies only after
  `alz account allow` in that repository. A Node CLI cannot change its parent shell's
  environment, so switching works through `alz run` or, as with mise, a shell hook. alz sets
  only each tool's account variable and never reads, copies or prints logins. Off by
  default. Design in LAYERING-SPEC §7.13; work in TASK-50.
- **Public history starts at one clean commit.** Earlier local history held a private work
  order. Local backup branches are never pushed; push only `main`.
- **Releases come only from CI.** Trusted publishing (OIDC) instead of npm tokens, provenance
  on every version, and staged releases approved with 2FA. Agents are denied `npm publish`,
  `deprecate`, `dist-tag`, `token` and `login` by this repository's layer.

## Versioning
While in 0.x, a change to the `layer.yaml` format or to what `alz build` generates bumps the
minor version. 1.0 is declared once the layer format is stable.

## Maintenance
Weekly issue triage; security reports acknowledged within 72 hours; adapters re-verified when
Claude Code or Codex ships a minor version (`alz doctor` warns outside the verified range).
