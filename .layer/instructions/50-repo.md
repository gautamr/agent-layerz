# agent-layerz

agent-layerz (CLI `alz`) composes layered coding-agent configuration into repositories.
It is being ported to idiomatic Rust under `crates/` (decision-21, decision-22, plan doc-5,
architecture doc-6, milestone m-7). The TypeScript implementation (Node 22+, `src/`) is the
frozen behavioural reference until cutover and is never changed, not even for fixes
(decision-30): every bug is fixed only in Rust, `docs/LAYERING-SPEC.md` states the intended
behaviour, and where the reference differs the acceptance suite marks the case as reference
drift. The test suite under `test/` is the oracle and keeps changing.

This repository is managed by `alz` itself. `AGENTS.md`, `CLAUDE.md`, `.claude/`, `.mcp.json`
and `layer.lock` are generated from `.layer/` and the personal layer: change `.layer/`, then
run `mise run alz:build`. Never edit generated files.

## Commands
| Purpose | Command |
| --- | --- |
| Tests: acceptance and reference projects | `mise run test` |
| Acceptance suite only; `ALZ_BIN=<absolute path>` picks the binary, else the TypeScript reference | `mise run test:acceptance` |
| Lint and type check | `mise run lint` |
| Bundle the CLI | `mise run build` |
| Install this checkout as one file at `~/.local/bin/alz` | `mise run install:local` |
| Build the Rust alz release binary | `mise run rust:build` |
| Install or update the Rust alz as one file at `~/.local/bin/alz-rs` (`ALZ_NAME=alz` replaces the Node one) | `mise run install:local:rust` |
| Rust workspace: fmt, clippy `-D warnings`, tests | `mise run rust:check` |
| Rust workspace, the full local set (check, doc, MSRV, deny, audit) | `mise run rust:ci` |
| Lint GitHub Actions workflows (actionlint, zizmor) | `mise run workflows:lint` |
| Rebuild this repo's agent config with the current code | `mise run alz:build` |
| Check the generated agent config is current | `mise run alz:check` |

## Map
- `docs/LAYERING-SPEC.md` is the contract. Change the spec in the same pull request as behaviour.
- `docs/target-facts.md` holds verified facts about Claude Code and Codex. Adapters may only
  rely on facts recorded there.
- `src/resolve` stack and sources, `src/merge` merge rules per item type, `src/adapters/<target>`
  native output and adoption, `src/adopt` adoption planning, `src/render` writing files,
  `src/cli` commands, `src/checks` leak guard and secret scan, `src/identity` git identity
  inspection, reports and local setup.
- `test/acceptance/NN-*.test.ts` with fixtures in `fixtures/NN-*`: the language-neutral
  suite. It spawns the binary under test (`ALZ_BIN`) and never imports `src/`
  (`test/guards/` enforces it).
- `test/reference/` holds the TypeScript-only cases (in-process runs, source scans); they are
  deleted at cutover. `test/spec/` holds the spec's formulas as test-side code.

## Invariants
- Content of the repository being built is untrusted: `layer.yaml`, `layer.lock`, remote layers.
  Validate it before it reaches the file system or a subprocess.
- Every write and delete goes through `insideRoot`. Outside the repo, alz writes only the layer
  cache (`~/.cache/agent-layerz`) and `${XDG_STATE_HOME:-~/.local/state}/agent-layerz/allowed.yaml`
  with its lock and temporary file, which only `alz account allow` writes, through
  `src/resolve/account-allow-writer.ts`. That file never lies inside the repo, an account folder
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
- Releases are published only by CI. Never run `npm publish` or change dist-tags locally.
