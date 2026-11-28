# agent-layerz

[![CI](https://github.com/gautamr/agent-layerz/actions/workflows/rust.yml/badge.svg)](https://github.com/gautamr/agent-layerz/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.100](https://img.shields.io/badge/rust-1.100-orange.svg)](Cargo.toml)

Write your coding-agent configuration once, in layers, and compose it into every repository.

Instructions, subagents, skills, hooks, permissions, MCP servers and model choices live in
**layers**: a public personal layer, a private client layer, the repo layer. `alz`
merges them with fixed, documented rules and writes the native files each agent reads
(`AGENTS.md`, `CLAUDE.md`, `.claude/`, `.mcp.json`) into the repository. The output is committed,
so CI, teammates and every account on your machine see the same setup.

```text
personal (public)  →  client (private)  →  repo (.layer/)  ──alz build──▶  AGENTS.md, CLAUDE.md, .claude/, .mcp.json
```

## Status

✅ works today · 🔜 planned · 🙋 help wanted

### Coding agents

| Coding agent | Instructions | Agents | Skills and commands | Hooks | Permissions | MCP servers | Models | Settings |
| --- | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: |
| Claude Code | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| OpenAI Codex CLI | 🔜 | 🔜 | 🔜 | 🔜 | 🔜 | 🔜 | 🔜 | 🔜 |
| Gemini CLI, GitHub Copilot, Cursor, OpenCode, others | 🙋 | 🙋 | 🙋 | 🙋 | 🙋 | 🙋 | 🙋 | 🙋 |

Each coding agent is an adapter: a pure function from the merged layers to that agent's native
files. [CONTRIBUTING.md](CONTRIBUTING.md#write-an-adapter-for-another-coding-agent) explains how
to write one.

### Features

| Area | Feature | Status |
| --- | --- | :-: |
| Layers | Personal, client and repo layers from `github:`, `git:` and `path:` sources, pinned in `layer.lock` | ✅ |
| Merging | Fixed merge rules for every item, with algebraic laws checked by property tests ([the algebra](docs/algebra.md)) | ✅ |
| Merging | Agent overlays that narrow tools and never widen them | ✅ |
| Safety | Permission floor: a deny rule, a floor mode or a locked hook cannot be loosened down the stack | ✅ |
| Safety | Leak guard and secret scan for public layers | ✅ |
| Build | Deterministic, byte-identical builds; `alz check` for CI; `alz diff` | ✅ |
| Build | Adoption of hand-written `.claude/`, `CLAUDE.md`, `AGENTS.md` and `.mcp.json` (`build --adopt`) | ✅ |
| Insight | `alz ls`, `alz explain`, `alz doctor`, `alz lint`; `alz ls --live` asks Claude Code what it loaded | ✅ |
| Identity | Commit identity declared by a layer and checked by `alz check` | ✅ |
| Accounts | `alz account which` and `alz account allow` | ✅ |
| Accounts | Starting a tool with the account a repository's layers name | 🔜 |
| Autonomy | Autonomy levels mapped onto each agent's permission modes | 🔜 |
| Install | From source | ✅ |
| Install | Release binaries, Homebrew and mise | 🔜 |

The version is 0.x: formats can still change between minor versions. The full design, merge
rules and acceptance tests are in [docs/LAYERING-SPEC.md](docs/LAYERING-SPEC.md).

## Why

- **One source per concern.** Personal standards, client context and repo specifics stop being
  copied into every repo and drifting apart.
- **Deterministic merges.** The more specific layer wins, collections combine, and security can
  only get stricter: a repo cannot remove a deny rule or a locked hook that a general layer set.
- **Independent of the account you are logged in with.** Everything is written at project scope,
  so switching between Claude Code accounts (`CLAUDE_CONFIG_DIR`) never changes what an agent
  sees. `alz doctor` reports user-level agents or skills that would shadow the built ones.
- **Reviewable and reproducible.** Generated files carry a header naming their sources, and
  `layer.lock` pins every layer's revision. `alz check` fails in CI when the generated files or
  the lock no longer match the layers.
- **Adopts what you already have.** `alz build --adopt` turns a repo's hand-written `.claude/`,
  `CLAUDE.md`, `AGENTS.md` and `.mcp.json` into layer sources with the same effect.

## Install

`alz` is one Rust binary with no runtime, for macOS and Linux.

### Homebrew 🔜

Available from the first release:

```sh
brew install gautamr/tap/alz
```

### mise 🔜

Available from the first release. [mise](https://mise.jdx.dev) installs the binary from the
GitHub release:

```sh
mise use -g github:gautamr/agent-layerz
```

### From source ✅

Works today. You need [mise](https://mise.jdx.dev), which installs the Rust toolchain this
repository pins, git 2.30 or newer and a POSIX shell.

```sh
git clone https://github.com/gautamr/agent-layerz.git
cd agent-layerz
mise trust                  # the first time: allow mise to use this repository's mise.toml
mise run install:local
alz --version
```

`mise run install:local` builds `alz` from the locked dependencies and installs it as one
regular file at `~/.local/bin/alz`, never a symlink. It prints the version, the git revision and
the SHA-256 of the installed file, and warns when the folder is not on your `PATH`. To install
somewhere else, set `PREFIX` (installs to `$PREFIX/bin`), `BIN_DIR` or `ALZ_NAME`:

```sh
BIN_DIR=~/bin mise run install:local        # installs ~/bin/alz
ALZ_NAME=alz-next mise run install:local    # installs ~/.local/bin/alz-next
```

To update, run `git pull` in the clone and `mise run install:local` again. It replaces the
installed file in place and prints the version it replaced.

### Uninstall

```sh
rm ~/.local/bin/alz                                          # or wherever you installed it
rm -rf "${XDG_CACHE_HOME:-$HOME/.cache}/agent-layerz"        # the layer cache
rm -rf "${XDG_STATE_HOME:-$HOME/.local/state}/agent-layerz"  # accounts accepted with `alz account allow`
```

The files `alz` generated in your repositories are ordinary committed files and stay where they
are.

## Quick start

In an existing repository:

```sh
alz init --stack github:<owner>/<layer-repo>@<tag>   # optional: --hub ../client-hub
alz build --adopt     # moves hand-written agent config into .layer/, then builds
alz ls                # every agent, skill, rule, hook and MCP server, and the layer it comes from
alz check             # in CI: fails when the generated files or the lock are out of date
```

`github:<owner>/<layer-repo>@<tag>` is your personal layer: a git repository, pinned to one of
its tags. `alz init --kind layer` in an empty folder scaffolds one. Use only layers you trust: when
the repository has no git identity of its own, `alz init` and `alz build --adopt` set it from
the layer's `commit_identity`, and the layer's hooks run as commands.
Without a personal layer yet, `alz init` alone gives a repo layer in `.layer/`; add more general
layers to `layer.yaml` later.

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
  - source: github:<owner>/<layer-repo>@<tag>
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
| `alz doctor` | Agents and skills in account folders that collide with the built ones, the installed Claude Code version against the verified range, files Claude Code also loads from folders around the repository, and the build's own diagnostics. |
| `alz account which` | The account each target (Claude Code, Codex) would use in this directory, where it comes from and why each other source was skipped; read-only. Exits 1 when a named account cannot apply. |
| `alz account allow [--remove]` | Accept (or withdraw) the account this repository's layers name: shows what it would record and asks you to type each account name. Only a person at a terminal outside an agent session can run it; there is no `--yes`. Writes `${XDG_STATE_HOME:-~/.local/state}/agent-layerz/allowed.yaml`. Until it runs, a layer-named account does not apply; `alz build` and `alz check` print the command on a terminal. |
| `alz lint [--layer <path>]` | Validate a layer (`--layer`), or this repository's stack and lock: schema, leak guard for public layers, slots. |

## Safety

- `alz` writes only inside the repository and its layer cache,
  `${XDG_CACHE_HOME:-~/.cache}/agent-layerz`. The one exception is `alz account allow`, which
  writes `allowed.yaml` in `${XDG_STATE_HOME:-~/.local/state}/agent-layerz`. That file may never
  lie inside a repository, an account folder or a git work tree, and neither may its real path
  when it is a link. Missing parent folders are created with mode 0700.
- It never overwrites a file it did not generate: that is a collision, unless you pass `--adopt`.
- The only git config it writes is the missing `user.name` and `user.email` in the repository's
  own `.git/config`, by `init` and `build --adopt`; never a global or system file.
- It never reads credentials or `.env` files, and never stores secret values: MCP servers name
  environment variables, not values.
- Public layers are scanned for secrets and for the `forbid_patterns` they declare, so private
  names do not leak into a public layer.
- Remote layers (`github:` and remote `git:` URLs) are cloned with the local and `file`
  transports blocked, so a repository cannot make them read a folder inside itself. If your
  `url.<base>.insteadOf` rewrites such a URL to a local mirror path, the clone fails; name
  the mirror as a local `git:` path instead.
- No layer may set `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `HOME`, `XDG_CONFIG_HOME`,
  `XDG_STATE_HOME` or `XDG_CACHE_HOME` through any of:
  - its settings;
  - an MCP server's `env_from`;
  - an agent's native MCP servers;
  - a passthrough settings, `.mcp.json`, agent or `.toml` file, under any target.

  The build fails and names the layer, never the value. This covers declarative configuration
  only; hooks and MCP commands run code and are not checked. Passthrough paths must be ASCII.
- Layers can ship hooks, which your agent runs as commands. Use layers you trust, and pin remote
  ones to a tag or commit.

## The algebra

Every merge in `alz` is an algebraic structure with written laws, and a property test checks
each law on thousands of generated inputs. A stack of layers is a fold:

```text
v = e ⊕ v₁ ⊕ v₂ ⊕ … ⊕ vₙ        layers from most general to most specific; e: "says nothing"
```

| Structure | Merge `a ⊕ b` | Laws | Used for |
| --- | --- | --- | --- |
| Last-writer-wins | `b` when set, else `a` | monoid: associative, with identity | an agent's scalar fields; skills, commands and MCP servers by name |
| Ordered union | `a ∪ b`, first occurrence kept | idempotent monoid; commutative as a set | `allow`, `ask` and `deny` lists, hook order |
| List edit | `+[…]`, `-[…]`, `[…]` composed | monoid acting on lists: `apply(a ⊕ b) = apply(b) ∘ apply(a)` | an agent's tools and skills |
| Deep merge | maps per key, lists by union, else `b` | associative on kind-consistent trees | settings |
| Floor | `max(a, b)` on a strictness order | join-semilattice; monotone: `a ⊕ b ≥ a` | permission modes, locked hooks |
| Narrowing | `a ≽ b`: `a` grants all `b` does | preorder: reflexive, transitive | overlays may only narrow tools |

These laws are what you rely on:

- Associativity: a hub's layers merge the same whether they are merged on their own first or
  inside the full stack.
- Identity: a layer that says nothing changes nothing.
- Idempotence: a repeated rule changes nothing.
- Monotonicity: security only gets stricter down the stack.
- Determinism: same inputs, byte-identical files, so `alz check` is an equation.

[docs/algebra.md](docs/algebra.md) has each law, its property test, the one known exception
(deep merge when a path changes kind) and a reading list, from property-based testing to
lattices, CRDTs and reproducible builds.

## Contributing

Contributions are welcome, especially adapters for more coding agents.
[CONTRIBUTING.md](CONTRIBUTING.md) covers building from source, the checks, the algebra with a
worked example, how to write an adapter, and how changes are proposed. Use the issue forms for
bugs, feature requests and adapter proposals.

## License

[MIT](LICENSE)
