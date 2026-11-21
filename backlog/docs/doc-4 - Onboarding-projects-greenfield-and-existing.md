---
id: doc-4
title: 'Onboarding projects: greenfield and existing'
type: guide
created_date: '2026-11-20T16:57:00Z'
updated_date: '2026-11-20T16:58:00Z'
tags:
  - guide
  - onboarding
---
# Onboarding projects: greenfield and existing

## Readiness checklist (both)
| Check | Command | Pass when |
| --- | --- | --- |
| alz installed | `alz --version` | prints the released version (install: `npm install -g --prefix ~/.local agent-layerz`) |
| Node | `node --version` inside the project | 22 or newer |
| git | `git status --short` | every repository is git, and work in progress is committed or stashed |
| account | the project's mise config | the intended Claude account folder is selected |
| account folders | `alz doctor` (after `alz init`; it needs layer.yaml) | no name collisions with the layers |
| client root | `ls <client-root>` | no CLAUDE.md or AGENTS.md at the client root; the root is not a git repo |
| ownership | — | decided whether generated agent config may be committed to the repositories (client-owned repositories may not allow it) |

## Greenfield
1. Personal layer: reuse it (`github:<you>/agent-layer-personal@<tag>` or a local path pinned to a tag).
2. Multi-repository client: create the hub first, so every repository picks up its layer.
   `mkdir -p <root>/hub/layer && cd <root>/hub && git init`, then `cd layer && alz init --kind layer --name <client> --visibility private`,
   then commit and tag `layer-v0.1.0`. Single repository: skip the hub and use `.layer/` only.
3. Each repository: `alz init --stack <personal source>@<tag> --hub ../hub`, then `alz build`, `alz check`, commit.
4. Trust each repository once in Claude Code, then `alz ls --live`.
5. Add project facts to `.layer/instructions`, path rules to `.layer/rules`, and profiles to `docs/standards/`.

## Existing projects
1. Commit or stash work in progress; create a branch (`git switch -c chore/alz`).
2. `alz init --stack <personal source>@<tag> [--hub ../hub]`.
3. `alz build --adopt`: hand-written `.claude/`, CLAUDE.md, AGENTS.md, settings.json and .mcp.json become
   repo-layer sources with the same effect. If any part cannot be adopted safely, nothing changes and every
   problem is listed.
4. `alz check`, review the diff, commit (or keep it local, see below).
5. Shrink large instruction files: a CLAUDE.md of hundreds of lines is adopted whole as a Claude-only
   block; split it into client facts (client layer), a standards profile that refines a personal skill
   instead of repeating it, and path rules.
6. Agents shared by sibling repositories move from a repository's `.layer/` to the client layer
   (by hand today; `alz promote` is planned).

## Shared or local
- Shared: commit `layer.yaml`, `layer.lock`, `.layer/` and the generated files; the whole team gets the agents.
- Local: keep them out of git with `.git/info/exclude`. This works for new files only; a file the team
  already tracks (for example CLAUDE.md) still shows as changed. A first-class private mode is a draft.

## Lessons from the first two client rollouts (anonymised)
- Client A: a hub with three code repositories and an earlier hand-built harness. Migrated with layers;
  everything checks. Unpinning the client layer while it changes often keeps the loop simple.
- Client B: four repositories in different stacks (Android, TypeScript, Terraform), no hub, many
  uncommitted changes, one 762-line CLAUDE.md and a drifted GEMINI.md, repositories owned by the client's
  organisation. A dry run adopted cleanly; the blockers were decisions (shared or local, a hub), not tooling.
