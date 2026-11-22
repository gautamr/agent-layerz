---
id: doc-5
title: Rust port plan
type: specification
created_date: '2026-11-22T12:53:00Z'
updated_date: '2026-11-22T16:30:00Z'
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

0. Idiomatic Rust, no compromise (decision-22). The TypeScript code is a reference for
   behaviour only, never for structure: no module layout, type shape, error style, naming or
   pattern is carried over because the TypeScript code had it. The Rust code is designed from
   the spec as Rust, to the standard below.
1. The spec is the contract; the suite is the oracle. Behaviour changes only through the spec.
2. Parity before features: no new behaviour in either implementation until cutover, except fixes
   both must carry.
3. Byte-identical generated files: for every fixture, the Rust binary writes the same generated
   files and `layer.lock` as the TypeScript one, because they are committed in users' repositories.
   Messages follow the spec, not the TypeScript code (choice 5, decision-26).
4. One slice at a time, in the order data flows, each slice passing its share of the suite. The
   phases split the behaviour to port, not the code: crate and module boundaries come from the
   Rust design in R0, not from `src/`.
5. Same security rules: untrusted repository content, every write through a guarded root, no git
   or network in account resolution, nothing read inside account folders.
6. Where Rust can close an accepted Node limit (`openat`-relative access, component-by-component
   symlink walks), it does, and the spec Limit text is narrowed in the same change.

## Rust standard (set in R0, enforced from the first commit)

The full standard is the personal skill `gr-rust` (agent-layerz-personal), with type-level design, errors and effects, layout and tooling, testing and anti-patterns; the layout and domain modelling for alz are in doc-6. The summary:


- Rust 1.100.0 pinned through mise, edition 2024, MSRV equal to the pin and raised with it
  (decision-25). `rustfmt` with no custom style.
- `clippy` with `-D warnings` and the `pedantic` group, plus restriction lints chosen in R0
  (`unwrap_used`, `expect_used`, `panic` outside tests, `print_stdout` outside the output layer).
  Any `allow` names its reason.
- `unsafe_code = "forbid"` in `[workspace.lints.rust]`, inherited by every crate. File-system safety (`openat`, `O_NOFOLLOW`) comes
  from `rustix` 1.1.6 or later, called only inside `alz-io` (decision-27).
- Types carry the invariants: newtypes for validated values (account names, layer names, object
  ids, sources, safe relative paths) built only by parsing ("parse, don't validate"). Enums, not
  strings, for kinds and states; exhaustive `match`; no stringly typed maps where a struct fits.
- Errors are typed: `thiserror` enums in library code, one error type per boundary, conversion
  with `From`, context at the edges. No `anyhow` in library crates. User-facing diagnostics are
  rendered in one place.
- Ownership-first APIs: borrow by default, no needless `clone`, no `Rc<RefCell<_>>` designs,
  iterators over index loops. Pure functions for merge and resolution; I/O at the edges behind
  small traits only where tests need a seam.
- Synchronous code. No async runtime unless a real need appears and is recorded as a decision.
- CLI with `clap` derive. Exit codes and output stay as the spec defines them; where a Rust
  convention (clap's usage errors, help layout) changes user-visible text, the spec is updated
  on purpose, not matched by bending the code (decision-26).
- Docs: `missing_docs` on public items through the workspace lints (CI runs with `-D warnings`), rustdoc examples that run as doc tests.
- Tests: unit tests next to the code, integration tests in `tests/`, CLI tests with
  `assert_cmd`, and property tests (`proptest`) for parsers and merge rules. The TypeScript
  acceptance suite is a temporary external oracle during the port; it is replaced by Rust tests
  before cutover (R8).
- Supply chain: `cargo deny` (licences, advisories, duplicate versions) and `cargo audit` in CI;
  every new dependency is a recorded decision.
- The repository rules in `.layer/` get a Rust section in R0, and the TypeScript rules stop
  applying to new code.

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
| R7 | Distribution | Prebuilt binaries for macOS arm64 and x86_64 and static musl Linux x86_64 and arm64 (decision-25), GitHub releases, Homebrew, an npm wrapper so `npm install -g agent-layerz` keeps working, local install as one regular file; replaces TASK-4, TASK-15, TASK-16 |
| R8 | Cutover | Whole suite green on Rust, parity run on every fixture, TypeScript removed, docs and `.layer/` updated |

## Choices settled in R0 (owner)

The owner answered on 2026-11-22 (TASK-55.1.2). The options and trade-offs are in that task's notes.

1. Repository layout and 6. crate layout, decision-23: a Cargo workspace at the root of this
   repository, next to the TypeScript code until cutover. The crates are those of doc-6 (alz-core,
   alz-io, alz, alz-test-support). The binary is also banned from `std::fs`, `std::process` and
   `std::env`, so only `alz-io` performs effects. `xtask` and `fuzz` are added when they are first
   needed.
2. YAML library: not decided yet. `serde_yaml` is unmaintained; candidates must support alias
   limits and give errors without quoting content. The spike is TASK-55.1.1, and its result is
   recorded as a decision.
3. Git access, decision-24: the `git` binary with argument arrays and an explicit, guarded child
   environment; no git library.
4. MSRV and targets, decision-25: Rust 1.100.0 pinned through mise, with the MSRV equal to the
   pin. Targets are macOS arm64 and x86_64, and Linux x86_64 and arm64 as static musl binaries.
   Windows stays undecided.
5. Error and output text, decision-26: the spec owns streams, the diagnostic format, exit codes
   and the messages it quotes. clap's defaults own help, usage errors and `--version`, and alz
   renders and sanitises clap's errors. The spec gains an "Output and exit codes" section in a
   later task.
6. Safe directory access, decision-27: `rustix` 1.1.6 or later only, inside `alz-io` capability
   types, with alz's own walk that never follows a link.

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
