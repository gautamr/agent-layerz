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
mise trust      # the first time: allow mise to use this repository's mise.toml
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
| Your working copy without installing it | `mise run rust:build` in the clone, then `<clone>/target/release/alz <command>` from any folder |

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

`mise run test` and `mise run lint` run the two halves of `rust:check` on their own.

CI runs the same mise tasks. On macOS and Linux it runs `rust:check` and `rust:doc`. On Linux it
also runs `rust:msrv`, `rust:deny`, `rust:audit`, `alz:check -- --skip-identity` and
`workflows:lint`. Before a pull request, run `rust:ci` and `alz:check`, and `workflows:lint`
when you changed a workflow.

## This repository builds itself

`alz` manages this repository's own agent configuration. `AGENTS.md`, `CLAUDE.md`, `.claude/`,
`.mcp.json` and `layer.lock` are generated from two layers: the public personal layer, which
`layer.yaml` names as a pinned `github:` source, and the repo layer in `.layer/`.

- Never edit a generated file. Change `.layer/`, then run `mise run alz:build`. Commit the
  source change and the generated files together.
- `alz:build` and `alz:check` build `alz` from your working copy first, so a change to the code
  can change the generated files too. `alz:check` fails when they are out of date.
- The first build fetches the personal layer into the layer cache,
  `${XDG_CACHE_HOME:-~/.cache}/agent-layerz`, which needs network access. Later builds use the
  cache.
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
| `fixtures/` | The input repositories of the acceptance suite (spec §13), with their layers. |
| `crates/alz/tests/fixtures/golden/` | The files each of those repositories must build to; its README says how they are kept. |
| `docs/LAYERING-SPEC.md` | The contract: what `alz` must do. |
| `docs/algebra.md` | The merge structures, their laws and the tests that check them. |
| `docs/spec-coverage.md` | Which test proves each requirement of the spec; a test fails when it is out of date. |
| `backlog/decisions/` | Recorded decisions, cited in the code and the spec as `decision-N`. |
| `docs/target-facts.md` | Verified facts about Claude Code and Codex. |

Code follows the rules in `.claude/rules/rust.md`: a functional style, typed errors and effects
kept at the edges, in `alz-io`. A new dependency needs a recorded decision first (the
`record-decision` skill).

## The spec, tests and target facts

- **The spec is the contract.** A change in behaviour changes `docs/LAYERING-SPEC.md` in the same
  pull request.
- **Tests come from the spec.** A test asserts what a section of the spec requires, not what the
  code happens to do. Write it first and watch it fail. Unit tests sit beside the code, and
  integration and command tests are in `crates/*/tests/`. The acceptance tests of spec §13 build
  whole repositories: the inputs are in `fixtures/NN-<topic>/`, and the expected output is in
  `crates/alz/tests/fixtures/golden/`. The `spec-test` skill in `.claude/skills/spec-test/` says
  which kind of test fits a requirement and how to write it, and `docs/spec-coverage.md` must
  name the new test.
- **Adapters rely only on recorded facts.** An adapter may depend on how Claude Code or Codex
  behaves only when `docs/target-facts.md` records that behaviour, with how it was checked:
  SMOKE (observed by running the tool), HELP (its `--help`), DOCS (vendor documentation, cited)
  or UNVERIFIED (do not build on it). The `verify-target-fact` skill describes the method. Never
  write to `~/.claude*`, `~/.codex*` or another account folder to run a check.
- **Repository content is untrusted.** `layer.yaml`, `layer.lock` and remote layers are
  validated before they reach the file system or a subprocess. A change that reads layer
  content, writes or deletes files, runs git or handles hooks, permissions or secrets gets a
  security review; the `threat-model` skill has the checklist.

## The algebra in practice

Every value a layer can set merges through a small algebraic structure with written laws:
last-writer-wins, a union, a list edit, a deep merge, or a floor that only gets stricter.
[docs/algebra.md](docs/algebra.md) states each structure, its laws and the property test that
checks them. When you add something a layer can set, first choose the structure it merges with.

Say a new setting must only get stricter down the stack, as permission modes do. Order its
values from loosest to strictest and use `Floor`, whose merge is `max`:

```rust
use alz_core::merge::{Floor, OrderedSet};

/// How much a session may do on its own, loosest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Autonomy {
    Free,
    Ask,
    ReadOnly,
}

#[test]
fn a_repo_layer_cannot_loosen_what_the_personal_layer_set() {
    let personal = Floor::at(Autonomy::Ask);
    let repo = Floor::at(Autonomy::Free);
    assert_eq!(personal.merge(repo).get(), Some(&Autonomy::Ask));
}

#[test]
fn deny_lists_only_grow() {
    let personal: OrderedSet<&str> = ["read:.env*"].into_iter().collect();
    let repo: OrderedSet<&str> = ["bash:rm -rf *", "read:.env*"].into_iter().collect();
    assert_eq!(personal.merge(repo).as_slice(), ["read:.env*", "bash:rm -rf *"]);
}
```

The examples prove two cases. A law covers every case, so state it as a property and let
[proptest](https://proptest-rs.github.io/proptest/) generate the inputs:

```rust
use proptest::prelude::{prop_assert, proptest, Just, Strategy};

fn floors() -> impl Strategy<Value = Floor<Autonomy>> {
    proptest::option::of(proptest::prop_oneof![
        Just(Autonomy::Free),
        Just(Autonomy::Ask),
        Just(Autonomy::ReadOnly)
    ])
    .prop_map(|level| level.map_or_else(Floor::default, Floor::at))
}

proptest! {
    #[test]
    fn a_later_layer_never_lowers_the_floor(earlier in floors(), later in floors()) {
        prop_assert!(earlier.clone().merge(later) >= earlier);
    }
}
```

When no existing structure fits, write a new one in `crates/alz-core/src/merge/algebra/`:

1. Give it a `merge(self, later)` method, where `later` is the more specific value.
2. State its laws in its doc comment, and its exceptions too: `Tree::deep_merge` is associative
   only over kind-consistent trees, and its tests show the counterexample.
3. Give each law a property test: at least associativity and the identity, plus idempotence,
   commutativity or monotonicity where the rule needs them.
4. State the merge rule in `docs/LAYERING-SPEC.md` §7 and add the structure to docs/algebra.md.

## Write an adapter for another coding agent

`alz` turns one neutral model into each coding agent's native files through an adapter. The
Claude Code adapter is complete. Adapters for the others are wanted, and each one makes every
layer work in one more tool:

| Coding agent | Adapter |
| --- | --- |
| Claude Code | ✅ Complete (`crates/alz-target-claude`) |
| OpenAI Codex CLI | 🔜 Planned; its facts are partly recorded in `docs/target-facts.md` |
| Gemini CLI, GitHub Copilot, Cursor, OpenCode, Windsurf and others | 🙋 Wanted |

An adapter is a pure function from the model to files. It depends only on the `alz-adapter`
crate, so it cannot do I/O or reach core internals. To write one:

1. Open an issue with the **Adapter for a coding agent** form. It asks for the agent's native
   files and the facts you can check.
2. Record those facts in `docs/target-facts.md` with their evidence level. The
   `verify-target-fact` skill describes the method.
3. Read the adapter contract in [spec §8](docs/LAYERING-SPEC.md#8-target-adapters) and the
   `Adapter` trait in `crates/alz-adapter`. Use `crates/alz-target-claude` as the example.
4. Write the adapter as a new `crates/alz-target-<agent>` crate, with tests from the spec.

## Work and pull requests

- Work is tracked on a [Backlog.md](https://github.com/MrLesk/Backlog.md) board in `backlog/`.
  Read and change it with the `backlog` CLI, never by editing its files. Pick a task, or open an
  issue to propose one. The issue forms are for a bug report, a feature request and an adapter
  for a coding agent. A feature request asks how the new setting merges.
- One task, one pull request. The pull request template lists the checks and asks for the
  commands you ran.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org) and end
  with the task ID, for example `feat(adopt): adopt hand-written hooks (task-12)`.
- Commit under your own name. The `Commits` section of the generated `AGENTS.md` names the
  owner's identity, because the owner's personal layer declares it. It applies only in the
  owner's clone: in yours, tell your coding agent to commit as you. An agent's commit ends with
  a `Co-Authored-By:` trailer naming the agent.
- Releases are published only by CI. Never publish, tag or yank a release from your machine.
- This repository is public. Never put client, employer or private project names, secrets or
  paths from your machine into code, tests, docs, tasks or commit messages.
