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

`alz` is one Rust binary with no runtime. Releases of ready-made binaries and a Homebrew formula
are being prepared; until the first one is published, build `alz` from a clone. This needs
[mise](https://mise.jdx.dev) (it installs the Rust toolchain this repository pins), git 2.30 or
newer and a POSIX shell (macOS or Linux):

```sh
git clone https://github.com/gautamr/agent-layerz.git
cd agent-layerz
mise run install:local
alz --version
```

The task builds `alz` from the locked dependencies and installs it as a regular executable at
`~/.local/bin/alz`. It creates no symlink. Set `PREFIX` (installs to `$PREFIX/bin`), `BIN_DIR` or
`ALZ_NAME` to choose another place or name. It prints the version, the git revision and the
SHA-256 of the installed file. Run it again after `git pull` to update.

### Leaving a 0.1.0 install behind

Version 0.1.0 was installed from the JavaScript package registry. That install leaves a symlink
named `alz` on your `PATH`, and `mise run install:local` refuses to replace a symlink. Remove the
old install with the tool that made it, delete the leftover `alz` link if one remains, then run
`mise run install:local`.
Layers and repositories built with 0.1.0 keep working, with the stricter checks that
[CHANGELOG.md](CHANGELOG.md) lists.

## Development

The Rust workspace is in `crates/`; [docs/LAYERING-SPEC.md](docs/LAYERING-SPEC.md) is the
contract. All commands run through mise, which pins the toolchain:

```sh
mise run rust:check   # format check, clippy with -D warnings, and every test
mise run rust:ci      # the full local set: check, rustdoc, MSRV, cargo-deny, cargo-audit
mise run rust:build   # build the release binary at target/release/alz
mise run alz:check    # fail if this repository's generated agent config is out of date
mise run alz:build    # rebuild this repository's agent config (needs the personal layer next to the clone)
```

## Quick start

In an existing repository:

```sh
alz init --stack github:<you>/agent-layerz-personal@v1.0.0   # optional: --hub ../client-hub
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
├── layer.yaml            name, kind, visibility (public | private | confidential), version, commit_identity
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
  - source: github:<you>/agent-layerz-personal@v1.0.0
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

## Commit identity

A person who owns the stack can declare who every commit is made as, in the `layer.yaml` of the
first layer (normally the personal one):

```yaml
commit_identity:
  name: Your Name
  email: you@example.com   # prefer a no-reply address in a public layer
```

- A private layer with `kind: client`, outside the repository (normally the hub's `layer/`), may
  declare its own identity, for example a client-issued email. It replaces the first layer's
  identity as a whole. The repo layer, group layers, public layers and a second client layer
  cannot declare one; the build fails and names the layer.
- With a declaration, `AGENTS.md` ends with a `Commits` section: author and committer are that
  identity, agent-made commits end with a `Co-Authored-By:` trailer, and nobody commits under a
  CI or shared identity. Bots that commit on the hosting service are exempt.
- `alz init` and `alz build --adopt` add `user.name` and `user.email` to the repository's own
  `.git/config` when they are missing. They never replace a value that is already there and
  never write `~/.gitconfig`. Plain `alz build` only warns when git would use another identity.
- Reports name where the wrong value comes from (layer, scope, file or variable), never the
  value itself, and print the command that fixes it.
- `alz check` fails on a mismatch. CI runners and people other than the owner run
  `alz check --skip-identity`.

## Commands

| Command | Does |
|---|---|
| `alz init [--stack <source>]… [--hub <dir>]` | Scaffold `layer.yaml` and `.layer/`. `--hub` adds the hub's client layer at its newest `layer-v*` tag. `--kind layer` scaffolds a new layer. With a declared commit identity it also sets the repo-local git identity. |
| `alz build [--adopt]` | Resolve, merge and write the generated files and `layer.lock`. `--adopt` first turns hand-written native config into repo-layer sources; if any part cannot be adopted safely, nothing changes. `--adopt` also sets the repo-local git identity; plain `build` warns when it differs from the declared one. |
| `alz check [--skip-identity]` | Rebuild in memory; fail if the repo or the lock differs, or the effective git identity differs from the declared commit identity. `--skip-identity` skips that last check. |
| `alz diff` | Show what the next build would change. |
| `alz ls [--live]` | List built items and the layer that decides each. `--live` asks Claude Code what it loaded (one short model call) and fails on anything missing. |
| `alz explain <type> <name>` | Every layer that contributed to an item, in order, and the merged result. |
| `alz doctor` | Name collisions in account dirs, installed tool version, slots, budgets. |
| `alz account which` | The account each target (Claude Code, Codex) would use in this directory, where it comes from and why each other source was skipped; read-only. Exits 1 when a named account cannot apply. |
| `alz account allow [--remove]` | Accept (or withdraw) the account this repository's layers name: shows what it would record and asks you to type each account name. Only a person at a terminal outside an agent session can run it; there is no `--yes`. Writes `${XDG_STATE_HOME:-~/.local/state}/agent-layerz/allowed.yaml`. Until it runs, a layer-named account does not apply; `alz build` and `alz check` print the command on a terminal. |
| `alz lint [--layer <path>]` | Validate a layer: schema, leak guard for public layers, slots. |

## Safety

- `alz` writes only inside the repository and its cache (`~/.cache/agent-layerz`), and, for
  `alz account allow` only, `allowed.yaml` in `${XDG_STATE_HOME:-~/.local/state}/agent-layerz`
  (never inside a repository, an account folder or a git work tree or, for a linked file, the
  directory of its real path, which passes the same checks; missing parent folders are created
  with mode 0700).
  It never overwrites a file it did not generate: that is a collision, unless you pass `--adopt`.
- The only git config it writes is the missing `user.name` and `user.email` in the repository's
  own `.git/config`, by `init` and `build --adopt`; never a global or system file.
- It never reads credentials or `.env` files, and never stores secret values: MCP servers name
  environment variables, not values.
- Public layers are scanned for secrets and for the `forbid_patterns` they declare, so private
  names do not leak into a public layer.
- Remote layers (`github:` and remote `git:` URLs) are cloned with the local and `file`
  transports blocked, so a repository cannot make them read a folder inside itself. If your
  `url.<base>.insteadOf` rewrites such a URL to a local mirror path, the clone now fails; name
  the mirror as a local `git:` path instead.
- No layer may set `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `HOME`, `XDG_CONFIG_HOME`,
  `XDG_STATE_HOME` or `XDG_CACHE_HOME` through its settings, an MCP server's `env_from`, an
  agent's native MCP servers or a passthrough settings, `.mcp.json`, agent or `.toml` file
  (under any target), and passthrough paths must be ASCII. The
  build fails and names the layer, never the value. This covers declarative configuration
  only; hooks and MCP commands run code and are not checked.
- Layers can ship hooks, which your agent runs as commands. Use layers you trust, and pin remote
  ones to a tag or commit.

## Status

Early (0.x). The Claude Code adapter is complete; Codex and other targets are planned. The full
design, merge rules and acceptance tests are in [docs/LAYERING-SPEC.md](docs/LAYERING-SPEC.md).
Also planned: optional account switching, where alz chooses each repository's Claude Code or
Codex account folder and never reads logins. The design is in
[LAYERING-SPEC §7.13](docs/LAYERING-SPEC.md#713-account-selection) (decision-15).

## License

MIT
