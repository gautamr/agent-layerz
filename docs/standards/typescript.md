# TypeScript profile: agent-layerz
Refines gr-typescript-domain for this repository.

## Read first
| Work | Files |
| --- | --- |
| Any behaviour change | `docs/LAYERING-SPEC.md` (the section for the item type) |
| Adapter work | `docs/target-facts.md`, `src/adapters/types.ts` |
| Anything touching repo content, files or git | the threat-model skill, `src/core/files.ts` |

## Decisions to preserve
- One bundled ESM file (`dist/cli.js`) with two runtime dependencies, `yaml` and `diff`.
- Node 22 is the oldest supported line: no APIs newer than Node 22.
- Commands take an `Io` (cwd, env, stdout, stderr) so tests run them in-process.
- Merge code is target-neutral; anything that knows a tool's native format lives in
  `src/adapters/<target>/`.
- Generated output is deterministic and committed by the repositories that use it.

## Overrides
- Overrides gr-typescript-domain §6 (typed `Result` for expected failures): the build reports
  every user error in one run, so expected failures are collected in `Diagnostics`
  (`diags.error(code, message, where)`) rather than returned one at a time. `InputError` is
  thrown only inside a parser and turned into a diagnostic by its caller; `FatalError` stops
  the command when it cannot continue.
- Overrides gr-typescript-domain §2 (domain modelling): the domain here is configuration
  merging, so "entities" are layers and items; no persistence or aggregates apply.
