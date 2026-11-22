---
id: doc-5
title: Rust port plan
type: specification
created_date: '2026-11-22T12:53:00Z'
updated_date: '2026-11-22T12:53:00Z'
---
# Rust port plan

decision-21: alz moves to Rust, and future tooling is written in Rust. Milestone m-7 holds the
work. This plan is the starting point; each phase is a task with its own plan and review.

## Starting point (2026-11-22)

- TypeScript implementation on `main` (commit `06bac7d`): about 10,600 lines in 103 files under
  `src/`, frozen as the reference. Runtime dependencies `yaml` and `diff`.
- Acceptance suite: 32 files, 840 cases, about 8,800 lines. Six files import source modules
  directly; the rest run the CLI.
- Contract: `docs/LAYERING-SPEC.md` (about 1,900 lines) and `docs/target-facts.md`.
- Baseline: `alz account which` about 70 ms per run, Node start about 25 ms (loaded machine).

## Principles

1. The spec is the contract; the suite is the oracle. Behaviour changes only through the spec.
2. Parity before features: no new behaviour in either implementation until cutover, except fixes
   both must carry.
3. Byte-identical output: for every fixture, the Rust binary writes the same generated files and
   `layer.lock` as the TypeScript one. Error and warning text matches what the tests assert.
4. One slice at a time, in the order data flows, each slice passing its share of the suite.
5. Same security rules: untrusted repository content, every write through a guarded root, no git
   or network in account resolution, nothing read inside account folders.
6. Where Rust can close an accepted Node limit (`openat`-relative access, component-by-component
   symlink walks), it does, and the spec Limit text is narrowed in the same change.

## Phases

| Phase | Task | Outcome |
| --- | --- | --- |
| R0 | Foundation and choices | Repository layout, toolchain through mise, crate choices recorded as decisions, CI job skeleton |
| R1 | Language-neutral suite | Every acceptance test drives a binary chosen by `ALZ_BIN`; the six source-importing files become CLI tests or move to TypeScript-only unit tests; TASK-54 temp-folder clean-up |
| R2 | Core model | Manifest, layer meta, lock, account model, YAML with size and alias limits, diagnostics, sanitising |
| R3 | Resolve | Sources (`path:`, `git:`, `github:` with the guarded clone), cache layout and integrity, safe file reads with `openat` |
| R4 | Merge | Every merge rule, protected variables, who may declare |
| R5 | Render, adapters and commands | Claude adapter, adoption, `init`, `build`, `check`, `diff`, `ls`, `explain`, `doctor`, `lint` |
| R6 | Identity and accounts | Commit identity, `alz account which`, `alz account allow` |
| R7 | Distribution | Prebuilt binaries (macOS and Linux), GitHub releases, Homebrew, an npm wrapper so `npm install -g agent-layerz` keeps working, local install as one regular file; replaces TASK-4, TASK-15, TASK-16 |
| R8 | Cutover | Whole suite green on Rust, parity run on every fixture, TypeScript removed, docs and `.layer/` updated |

## Choices to settle in R0 (owner)

1. Repository layout. Recommended: the same repository, a Cargo workspace at the root next to
   the TypeScript code until cutover, so the spec, fixtures and suite are shared.
2. YAML library. `serde_yaml` is unmaintained; candidates must support alias limits and give
   errors without quoting content. Record the choice as a decision after a spike.
3. Git access. Recommended: keep running the `git` binary with argument arrays and the same
   guarded environment, rather than a git library, so behaviour and the threat model stay the
   same.
4. Minimum supported Rust version and target platforms (macOS arm64 and x86_64, Linux x86_64 and
   arm64; Windows stays undecided as before).
5. Error and output text: identical to the TypeScript wording (recommended, so tests carry over).

## Paused TypeScript work

- TASK-50.3 to TASK-50.6 (run, doctor, init, use and the hook): designs stay in the spec and are
  built in Rust after cutover. The hook is built as a fast path from the start.
- TASK-52 (account-folder walk) and TASK-53 (empty or relative `XDG_CACHE_HOME`): built in Rust
  in R3, where `openat` closes them properly.
- TASK-54 (test temp folders): done in R1, because the suite is the oracle.
- TASK-4, TASK-15, TASK-16 (npm release pipeline): replaced by R7.
- Release gates for the account feature (the owner's manual `alz account allow` check, O5, O8)
  carry over to the Rust release.

## Personal layer

The personal layer's "TypeScript on Node.js for new projects" default changes for tooling to
Rust (mise-managed toolchain, Cargo workspaces). That change belongs in the personal layer
repository and its next tagged release.
