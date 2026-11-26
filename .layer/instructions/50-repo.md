# agent-layerz

agent-layerz (CLI `alz`) composes layered coding-agent configuration into repositories. It is
written in Rust under `crates/` (decision-21, decision-22, decision-39; architecture doc-6).
`docs/LAYERING-SPEC.md` states the intended behaviour, and Rust tests written from it prove it.

This repository is managed by `alz` itself. `AGENTS.md`, `CLAUDE.md`, `.claude/`, `.mcp.json`
and `layer.lock` are generated from `.layer/` and the personal layer: change `.layer/`, then
run `mise run alz:build`. Never edit generated files.

## Commands
| Purpose | Command |
| --- | --- |
| Tests: the Rust workspace's unit, integration and command tests | `mise run test` |
| Format check and clippy with `-D warnings` | `mise run lint` |
| Rust workspace: lint and tests | `mise run rust:check` |
| Rust workspace, the full local set (check, doc, MSRV, deny, audit) | `mise run rust:ci` |
| Build the Rust alz release binary | `mise run rust:build` |
| Install or update the Rust alz as one regular file at `~/.local/bin/alz` | `mise run install:local` |
| Lint GitHub Actions workflows (actionlint, zizmor) | `mise run workflows:lint` |
| Rebuild this repo's agent config with the current code | `mise run alz:build` |
| Check the generated agent config is current | `mise run alz:check` |

## Map
- `docs/LAYERING-SPEC.md` is the contract. Change the spec in the same pull request as behaviour.
- `docs/target-facts.md` holds verified facts about Claude Code and Codex. Adapters may only
  rely on facts recorded there.
- `crates/alz-core` is the pure domain: stack, merge rules per item type, the adapter registry
  and the model it hands adapters, adoption planning, rendering and the output checks, the leak
  guard and secret scan, identity checks. It has no file system, process or environment access.
- `crates/alz-adapter` is the adapter contract (spec §8.0, decision-40): the model, descriptor,
  output and the `Adapter` trait. `crates/alz-target-claude` is the Claude adapter; it depends
  on `alz-adapter` only, so it cannot reach core internals or do I/O.
- `crates/alz-io` holds every effect (files, git, the layer cache, the terminal, processes)
  behind capability types. `crates/alz` is the thin binary that wires them and owns the exit
  codes. `crates/alz-test-support` has the test helpers.
- Tests: unit tests beside the code, integration and command tests in `crates/*/tests/`, and
  fixtures in `fixtures/`. Write them from the spec with the spec-test skill.

## Invariants
- Content of the repository being built is untrusted: `layer.yaml`, `layer.lock`, remote layers.
  Validate it before it reaches the file system or a subprocess.
- Every write and delete goes through the repository-root capability. Outside the repo, alz writes only the layer
  cache (`~/.cache/agent-layerz`) and `${XDG_STATE_HOME:-~/.local/state}/agent-layerz/allowed.yaml`
  with its lock and temporary file, which only `alz account allow` writes, through
  the state-directory capability of `alz-io`. That file never lies inside the repo, an account folder
  or a git work tree or, for a linked `allowed.yaml`, the directory of its real path, which
  passes the same checks; missing parent folders are created with mode 0700. Account folders
  are read for names only.
- Builds are deterministic: same inputs, byte-identical output.
- Never overwrite a file the tool did not generate. Security only gets stricter down the stack.

## Work
- Work is tracked in Backlog.md in `backlog/`. At the start of a session run
  `backlog instructions overview`, and read the matching guide before creating, executing or
  finishing a task. Use the `backlog` CLI; never edit its files by hand. One exception, which
  the owner approved: the CLI sets only a decision's title, so fill the Context, Decision and
  Consequences of a decision created here by hand, in the same commit as its task. Never
  change a decision's title or status, or any other backlog file, by hand.
- One task, one pull request. Conventional Commits with the task ID, e.g. `feat(adopt): ... (task-12)`.
- Releases are published only by CI. Never publish, tag or yank a release locally.
