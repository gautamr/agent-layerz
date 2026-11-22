---
id: doc-5
title: Rust port plan
type: specification
created_date: '2026-11-22T12:53:00Z'
updated_date: '2026-11-22T17:16:00Z'
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
  (decision-25). `mise.toml` is the only source of the version; there is no
  `rust-toolchain.toml` (TASK-55.1.3). `rustfmt` with no custom style.
- `clippy` with `-D warnings` and the `pedantic` group, plus the restriction lints listed in
  `[workspace.lints.clippy]` of the root `Cargo.toml`, which every crate inherits. Among them:
  `unwrap_used`, `indexing_slicing`, `string_slice`, `panic_in_result_fn`, `unwrap_in_result`,
  `todo`, `unimplemented`, `dbg_macro`, `exit`, `as_conversions`, `print_stdout` and
  `print_stderr` (output goes through `std::io::Write` in the one place that renders it). The
  root `clippy.toml` allows unwrap, expect, printing and indexing in tests. `expect` with a
  message that says why the value is there is allowed (gr-rust), so `expect_used` and `panic`
  are not enabled. An outer `#[allow]` warns (`allow_attributes`), so a lint is silenced with
  `#[expect(..., reason = "...")]`, and every allow names its reason
  (`allow_attributes_without_reason`).
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
| R2 | Core model | Manifest, layer meta, lock, account model, YAML with size and alias limits (serde-saphyr into alz's own tree, and alz's own emitter, decision-28), diagnostics, sanitising (decision-26's text classes, moved from the binary into `alz-core`) |
| R3 | Resolve | Sources (`path:`, `git:`, `github:` with the guarded clone and the explicit child environment, decision-24), cache layout and integrity, safe file reads with `openat` and trust checks by `fstat` on the handle that is read (decision-27) |
| R4 | Merge | Every merge rule, protected variables, who may declare |
| R5 | Render, adapters and commands | Claude adapter, adoption, `init`, `build`, `check`, `diff`, `ls`, `explain`, `doctor`, `lint` |
| R6 | Identity and accounts | Commit identity, `alz account which`, `alz account allow` |
| R7 | Distribution | Prebuilt binaries for macOS arm64 and x86_64 and static musl Linux x86_64 and arm64 (decision-25), GitHub releases, Homebrew, an npm wrapper so `npm install -g agent-layerz` keeps working, local install as one regular file; replaces TASK-4, TASK-15, TASK-16 |
| R8 | Cutover | Whole suite green on Rust, parity run on every fixture, TypeScript removed, docs and `.layer/` updated |

## Choices settled in R0 (owner)

The owner answered on 2026-11-22. The options and trade-offs are in the notes of TASK-55.1.2, and
for YAML in TASK-55.1.1. On 2026-11-22 the owner also amended decision-24, decision-26,
decision-27 and decision-28 with tightenings from the R0 security review; the summaries below
include them.

1. Repository layout and 6. crate layout, decision-23: a Cargo workspace at the root of this
   repository, next to the TypeScript code until cutover. The crates are those of doc-6 (alz-core,
   alz-io, alz, alz-test-support). The binary is also banned from `std::fs`, `std::process` and
   `std::env`, so only `alz-io` performs effects. `xtask` and `fuzz` are added when they are first
   needed.
2. YAML library, decision-28, from the TASK-55.1.1 spike: `serde-saphyr` pinned `=2.2.0`,
   deserialisation only, with YAML 1.2 core options and limits (100 aliases, depth 64, 20000
   nodes, 10000 replayed events) and a 1 MiB cap per file before parsing. The cap and the limits
   apply to every YAML file alz reads, `accounts.yaml` and `allowed.yaml` included. alz reads
   into its own YAML tree and converts that tree in its own code. The conversion rejects every
   explicit tag outside the YAML 1.2 core set (`!!binary` and custom tags included) with its own
   error code, a stream of several documents, and control characters in strings after their
   escapes are decoded. Diagnostics come from the error variant and location only; the one piece
   of input they may hold is a key name, quoted and stripped as decision-26 class 1 says.
   Generated YAML comes from alz's own emitter, tested byte for byte against the TypeScript
   output. The fallback is `yaml_serde`. The parse changes users will see go into the spec
   later, and cutover needs a minor version bump. How a tag reaches alz through serde-saphyr is
   settled in R2.
3. Git access, decision-24: the `git` binary with argument arrays and an explicit, guarded child
   environment; no git library. For cache and clone operations the child environment drops
   `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_OBJECT_DIRECTORY`,
   `GIT_ALTERNATE_OBJECT_DIRECTORIES`, `GIT_COMMON_DIR`, `GIT_NAMESPACE` and
   `GIT_CEILING_DIRECTORIES`, so `alz check` run as a pre-commit hook cannot reach the user's
   index or repository. Revisions follow `--end-of-options`; `--` comes only before paths.
4. MSRV and targets, decision-25: Rust 1.100.0 pinned through mise, with the MSRV equal to the
   pin. Targets are macOS arm64 and x86_64, and Linux x86_64 and arm64 as static musl binaries.
   Windows stays undecided.
5. Error and output text, decision-26: the spec owns streams, the diagnostic format, exit codes
   and the messages it quotes. clap's defaults own help, usage errors and `--version`, and alz
   renders and sanitises clap's errors. Untrusted text is printed in one of three classes:
   stripped of control and format characters, line feed included, in diagnostics; escaped as
   printable ASCII with `\u{XXXX}` where a person decides on it, as in `alz account allow`; and
   inside clap's output, the only text that keeps its own line feeds, with the argument
   fragments stripped. Format characters are the whole Unicode category Cf. A panic prints a
   fixed message and exits 70. The spec gains an "Output and exit codes" section in a later
   task.
6. Safe directory access, decision-27: `rustix` 1.1.6 or later only, inside `alz-io` capability
   types. Their methods take validated relative paths (`SafeRelPath`) and walk them one
   component at a time relative to a directory handle, each component opened with `O_NOFOLLOW`,
   `O_DIRECTORY` and `O_CLOEXEC`. A link is followed only where the spec allows it, explicitly,
   with `readlinkat`, and each resolved prefix is checked (account folders). Every trust check
   (owner, mode, type, size) uses `fstat` on the handle that is then read; files are opened
   with `O_NONBLOCK` and must be regular files, and the 1 MiB cap is enforced by reading at most
   one byte more than the cap. Writes through links inside the repository are refused by
   default unless the spec decides otherwise in R5.

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
