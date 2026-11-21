# agent-layerz

agent-layerz (CLI `alz`) composes layered coding-agent configuration into repositories.
TypeScript on Node 22+, ESM, bundled to one file with esbuild. Runtime dependencies: `yaml`
and `diff` only.

This repository is managed by `alz` itself. `AGENTS.md`, `CLAUDE.md`, `.claude/`, `.mcp.json`
and `layer.lock` are generated from `.layer/` and the personal layer: change `.layer/`, then
run `mise run alz:build`. Never edit generated files.

## Commands
| Purpose | Command |
| --- | --- |
| Tests | `mise run test` |
| Lint and type check | `mise run lint` |
| Bundle the CLI | `mise run build` |
| Install this checkout as one file at `~/.local/bin/alz` | `mise run install:local` |
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
- `test/acceptance/NN-*.test.ts` with fixtures in `test/fixtures/NN-*`.

## Invariants
- Content of the repository being built is untrusted: `layer.yaml`, `layer.lock`, remote layers.
  Validate it before it reaches the file system or a subprocess.
- Every write and delete goes through `insideRoot`. Outside the repo, only the cache
  (`~/.cache/agent-layerz`) is written; account folders are read for names only.
- Builds are deterministic: same inputs, byte-identical output.
- Never overwrite a file the tool did not generate. Security only gets stricter down the stack.

## Work
- Work is tracked in Backlog.md in `backlog/`. At the start of a session run
  `backlog instructions overview`, and read the matching guide before creating, executing or
  finishing a task. Use the `backlog` CLI; never edit its files by hand.
- One task, one pull request. Conventional Commits with the task ID, e.g. `feat(adopt): ... (task-12)`.
- Releases are published only by CI. Never run `npm publish` or change dist-tags locally.
