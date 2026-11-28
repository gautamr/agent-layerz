# Contributing to agent-layerz

This guide is for people who change agent-layerz itself. To install and use `alz`, read the
[README](README.md).

## Set up

You need [mise](https://mise.jdx.dev), git 2.30 or newer and a POSIX shell (macOS or Linux).
mise installs everything else at the versions `mise.toml` pins: the Rust toolchain with rustfmt
and clippy, cargo-deny, cargo-audit, actionlint and zizmor.

```sh
git clone https://github.com/gautamr/agent-layerz.git
cd agent-layerz
mise install
mise run rust:check
```

Never install these tools globally for this repository. A global `cargo install` of cargo-deny
or cargo-audit is ignored: the tasks run the pinned binaries and fail if the versions differ.

## Use `alz` while you change it

Keep a stable `alz` for your own repositories and try your working copy under another name.

| You want | Run |
| --- | --- |
| A stable `alz` on your `PATH`, built from `main` | `git switch main && mise run install:local` |
| Your working copy installed as `alz-dev`, beside the stable one | `ALZ_NAME=alz-dev mise run install:local` |
| Your working copy without installing it | `cargo run -p alz -- <command>` |

`install:local` builds from the locked dependencies and installs one regular file at
`~/.local/bin/alz`, never a symlink. `PREFIX` (installs to `$PREFIX/bin`), `BIN_DIR` and
`ALZ_NAME` change the place and the name. Run it again to update. It prints the version, the git
revision (with `-dirty` for uncommitted changes) and the SHA-256 of the installed file.

Try `alz-dev` in a scratch repository first: `alz build` writes files into the repository it
runs in.

## Checks

| When | Run | What it does |
| --- | --- | --- |
| Before every push | `mise run rust:check` | Format check, clippy with `-D warnings`, every test |
| Before a pull request | `mise run rust:ci` | `rust:check`, rustdoc with warnings as errors, the MSRV check, cargo-deny and cargo-audit |
| After changing `.github/workflows/` | `mise run workflows:lint` | actionlint, and zizmor's security checks |
| After changing `.layer/` | `mise run alz:build`, then `mise run alz:check` | Rebuild this repository's agent config and check it is current |

`mise run test` and `mise run lint` run the two halves of `rust:check` on their own. CI runs
the same mise tasks on macOS and Linux, so a green local `rust:ci` means CI should pass too.

## This repository builds itself

`alz` manages this repository's own agent configuration. `AGENTS.md`, `CLAUDE.md`, `.claude/`,
`.mcp.json` and `layer.lock` are generated from two layers: the public personal layer, which
`layer.yaml` names as a pinned `github:` source, and the repository layer in `.layer/`.

- Never edit a generated file. Change `.layer/`, then run `mise run alz:build`. Commit the
  source change and the generated files together.
- `alz:build` and `alz:check` build `alz` from your working copy first, so a change to the code
  can change the generated files too. `alz:check` fails when they are out of date.
- The first build fetches the personal layer into `~/.cache/agent-layerz`, which needs network
  access. Later builds use the cache.
- `alz check` also compares your git identity with the one the personal layer declares. If you
  are not the owner, run `mise run alz:check -- --skip-identity`, as CI does.

## Code map

| Path | Holds |
| --- | --- |
| `crates/alz-core` | The pure domain: the stack, merge rules per item type, adoption planning, rendering, the output checks, the leak guard and secret scan, identity checks. No file system, process or environment access. |
| `crates/alz-adapter` | The adapter contract: the model, the descriptor, the output and the `Adapter` trait. |
| `crates/alz-target-claude` | The Claude Code adapter. It depends only on `alz-adapter`, so it cannot reach core internals or do I/O. |
| `crates/alz-io` | Every effect (files, git, the layer cache, the terminal, processes) behind capability types. |
| `crates/alz` | The thin binary: it wires the capabilities together and owns the exit codes. |
| `crates/alz-test-support` | Test helpers. |
| `fixtures/` | Whole repositories with their layers and the expected output. |
| `docs/LAYERING-SPEC.md` | The contract: what `alz` must do. |
| `docs/target-facts.md` | Verified facts about Claude Code and Codex. |

Code follows the rules in `.claude/rules/rust.md`: a functional style, typed errors and effects
kept at the edges, in `alz-io`.

## The spec, tests and target facts

- **The spec is the contract.** A change in behaviour changes `docs/LAYERING-SPEC.md` in the same
  pull request.
- **Tests come from the spec.** A test asserts what a section of the spec requires, not what the
  code happens to do. Write it first and watch it fail. Unit tests sit beside the code;
  integration and command tests are in `crates/*/tests/`; whole-repository cases are fixtures
  in `fixtures/NN-<topic>/`. The `spec-test` skill in `.claude/skills/spec-test/` says which kind
  of test fits a requirement and how to write it.
- **Adapters rely only on recorded facts.** An adapter may depend on how Claude Code or Codex
  behaves only when `docs/target-facts.md` records that behaviour, with how it was checked:
  SMOKE (observed by running the tool), HELP (its `--help`), DOCS (vendor documentation, cited)
  or UNVERIFIED (do not build on it). The `verify-target-fact` skill describes the method. Never
  write to `~/.claude*`, `~/.codex*` or another account folder to run a check.
- **Repository content is untrusted.** `layer.yaml`, `layer.lock` and remote layers are
  validated before they reach the file system or a subprocess. A change that reads layer
  content, writes or deletes files, runs git or handles hooks, permissions or secrets gets a
  security review; the `threat-model` skill has the checklist.

## Work and pull requests

- Work is tracked on a [Backlog.md](https://github.com/MrLesk/Backlog.md) board in `backlog/`.
  Read and change it with the `backlog` CLI, never by editing its files. Pick a task, or open an
  issue to propose one.
- One task, one pull request.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org) and end
  with the task ID, for example `feat(adopt): adopt hand-written hooks (task-12)`.
- Commit under your own name. An agent's commit ends with a `Co-Authored-By:` trailer naming
  the agent.
- Releases are published only by CI. Never publish, tag or yank a release from your machine.
- This repository is public. Never put client, employer or private project names, secrets or
  paths from your machine into code, tests, docs, tasks or commit messages.
