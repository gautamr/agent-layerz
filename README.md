# agent-layerz

Write your coding-agent configuration once, in layers, and compose it into every repository.

Instructions, subagents, skills, hooks, permissions, MCP servers and model choices live in
**layers**: a public personal layer, a private client layer, the repository's own layer. `alz`
merges them with fixed, documented rules and writes the native files each agent reads
(`AGENTS.md`, `CLAUDE.md`, `.claude/`, `.mcp.json`) into the repository. The output is committed,
so CI, teammates and every account on your machine see the same setup.

```text
personal (public)  →  client (private)  →  repo (.layer/)  ──alz build──▶  AGENTS.md, CLAUDE.md, .claude/, .mcp.json
```

## Why

- **One source per concern.** Personal standards, client context and repo specifics stop being
  copied into every repo and drifting apart.
- **Deterministic merges.** The more specific layer wins, collections combine, and security can
  only get stricter: a repo cannot remove a deny rule or a locked hook that a general layer set.
- **Independent of the account you are logged in with.** Everything is written at project scope,
  so switching between Claude Code accounts (`CLAUDE_CONFIG_DIR`) never changes what an agent
  sees. `alz doctor` reports user-level agents or skills that would shadow the built ones.
- **Reviewable and reproducible.** Generated files carry a header naming their sources, and
  `layer.lock` pins every layer's revision. `alz check` fails in CI when the two disagree.
- **Adopts what you already have.** `alz build --adopt` turns a repo's hand-written `.claude/`,
  `CLAUDE.md`, `AGENTS.md` and `.mcp.json` into layer sources with the same effect.

## Install

Node.js 22 or newer.

```sh
npm install --global agent-layerz      # or: mise use --global npm:agent-layerz
alz --version
```

## Quick start

In an existing repository:

```sh
alz init --stack github:<you>/agent-layer-personal@v1.0.0   # optional: --hub ../client-hub
alz build --adopt     # moves hand-written agent config into .layer/, then builds
alz ls                # every agent, skill, rule, hook and MCP server, and the layer it comes from
alz check             # in CI: fails when the generated files or the lock are out of date
```

Without a personal layer yet, `alz init` alone gives a repo layer in `.layer/`; add more
general layers to `layer.yaml` later.

## Layers

A layer is a directory, usually its own git repo. Every part is optional:

```text
<layer>/
├── layer.yaml            name, kind, visibility (public | private | confidential), version
├── instructions/*.md     concatenated into AGENTS.md, in stack order then file order
├── agents/<name>.md      subagents; <name>.overlay.md extends an agent from a general layer
├── skills/<name>/        Agent Skills (SKILL.md plus files); the most specific layer wins
├── commands/<name>.md    user-invoked skills
├── rules/<name>.md       path-scoped rules (frontmatter `paths:`)
├── hooks/hooks.yaml      hooks; `locked: true` stops later layers from disabling one
├── permissions.yaml      allow / ask / deny; deny rules can never be removed downstream
├── mcp.yaml              MCP server definitions; repos enable the ones they need
├── models.yaml           model aliases and role routing
├── settings.yaml         other native settings, by target
└── targets/claude/…      raw files copied as-is (escape hatch)
```

The repository lists its stack, most general first, in `layer.yaml`:

```yaml
stack:
  - source: github:<you>/agent-layer-personal@v1.0.0
  - source: path:../client-hub/layer
    ref: layer-v0.3.0          # read from this git tag, not the working tree
  - source: path:./.layer      # the repo layer, always last
targets: [claude]
workspace:
  hub: ../client-hub           # shared docs and specs, granted as an additional directory
mcp:
  enable: [context7]
```

Agents are written in a neutral schema. Fields only one tool understands go under its name:

```markdown
---
name: architect
description: Plans slices and owns the API contract.
tools: [read, grep, glob, write, edit, bash]
model: strong                  # an alias from models.yaml
memory: project
claude:
  effort: xhigh
  maxTurns: 60
---
You are the architect…
```

Hooks either ship a script inside the layer (`run: scripts/guard.sh`) or run a command as
written (`command: '"${PROJECT_DIR}"/scripts/check.sh'`). Events are neutral (`pre_tool`,
`post_tool`, `stop`, …) or a tool's own (`claude:Notification`).

## Commands

| Command | Does |
|---|---|
| `alz init [--stack <source>]… [--hub <dir>]` | Scaffold `layer.yaml` and `.layer/`. `--hub` adds the hub's client layer at its newest `layer-v*` tag. `--kind layer` scaffolds a new layer. |
| `alz build [--adopt]` | Resolve, merge and write the generated files and `layer.lock`. `--adopt` first turns hand-written native config into repo-layer sources; if any part cannot be adopted safely, nothing changes. |
| `alz check` | Rebuild in memory; fail if the repo or the lock differs. |
| `alz diff` | Show what the next build would change. |
| `alz ls [--live]` | List built items and the layer that decides each. `--live` asks Claude Code what it loaded (one short model call) and fails on anything missing. |
| `alz explain <type> <name>` | Every layer that contributed to an item, in order, and the merged result. |
| `alz doctor` | Name collisions in account dirs, installed tool version, slots, budgets. |
| `alz lint [--layer <path>]` | Validate a layer: schema, leak guard for public layers, slots. |

## Safety

- `alz` writes only inside the repository and its cache (`~/.cache/agent-layerz`). It never
  overwrites a file it did not generate: that is a collision, unless you pass `--adopt`.
- It never reads credentials or `.env` files, and never stores secret values: MCP servers name
  environment variables, not values.
- Public layers are scanned for secrets and for the `forbid_patterns` they declare, so private
  names do not leak into a public layer.
- Layers can ship hooks, which your agent runs as commands. Use layers you trust, and pin remote
  ones to a tag or commit.

## Status

Early (0.x). The Claude Code adapter is complete; Codex and other targets are planned. The full
design, merge rules and acceptance tests are in [docs/LAYERING-SPEC.md](docs/LAYERING-SPEC.md).

## License

MIT
