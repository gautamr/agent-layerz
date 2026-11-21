---
id: doc-1
title: Publication plan
type: specification
created_date: '2026-11-19T17:10:00Z'
updated_date: '2026-11-19T17:10:00Z'
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
   account trust checks with workspace-accounts, a decision on Windows.
5. **Phase 4: Launch.** An anonymised case study from a real multi-repo client migration, a
   launch post with an honest comparison, a short terminal recording, announcements.

## Decisions
- **This repository is built with alz.** Its agents, skills, rules, hooks, permissions and MCP
  servers live in `.layer/` on top of the owner's public personal layer. Gaps found while
  doing so become tasks here.
- **Account management stays in a separate project, workspace-accounts.** A Node CLI cannot
  change the parent shell's environment; switching accounts is a shell and mise concern; and
  alz promises never to touch credentials or home folders. The two meet in a small contract:
  account folders hold logins only, both use the same folder discovery, and trust is per
  account.
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
