---
id: doc-6
title: 'Rust architecture: layout, domain modelling and type-level design'
type: specification
created_date: '2026-11-22T14:38:00Z'
updated_date: '2026-11-22T16:30:00Z'
---
# Rust architecture: layout, domain modelling and type-level design

Companion to doc-5 (plan), decision-21 (Rust) and decision-22 (idiomatic Rust). The layout in §2
was adopted with two changes in decision-23 (TASK-55.1.2). Git access is decision-24, the
toolchain and targets decision-25, output text decision-26, and directory access decision-27.
Nothing here mirrors `src/`; it starts from what alz is.

## 1. What alz is, for design purposes

alz is a **deterministic compiler with a few guarded effects**:

- *Pure core*: parse manifests and layers, resolve a stack, merge items by fixed rules, render
  native files into an in-memory tree, decide accounts and identities. Same input, same bytes.
- *Effects at the edge*: read layer files, run `git`, read and write the cache, write generated
  files inside the repository, write `allowed.yaml`, read the terminal and environment.
- *Hostile input*: repository content, locks and remote layers are untrusted.

So the architecture is "functional core, imperative shell", and the type system does most of the
safety work: values are parsed once into types that cannot hold invalid data, and effects are
reachable only through capability types.

## 2. Repository layout

Cargo workspace in this repository, next to the frozen TypeScript code until cutover (R8). The
names do not collide: Rust lives under `crates/`, TypeScript under `src/` and `test/`.

```text
agent-layerz/
├── Cargo.toml                 # [workspace]: members = ["crates/*"] (+ "xtask" once it exists), resolver = "3"
│                              # [workspace.package]: edition = "2024", rust-version = the pin, license
│                              # [workspace.lints]: rust + clippy levels, inherited by every crate
│                              # [workspace.dependencies]: one version per dependency
├── Cargo.lock                 # committed: alz ships a binary
├── rust-toolchain.toml        # if kept: the same version as the mise pin (1.100.0, decision-25),
│                              # components rustfmt and clippy; TASK-55.1.3 checks mise and rustup agree
├── deny.toml                  # cargo-deny: licences, advisories, bans, duplicate versions
├── clippy.toml                # thresholds and test allowances; MSRV comes from Cargo.toml. A crate's
│                              # own clippy.toml replaces this file (not merged): repeat the allowances
├── crates/
│   ├── alz-core/              # library, no I/O: the domain and its algebra
│   │   ├── clippy.toml        # disallowed-methods: std::fs, std::process, std::env
│   │   ├── src/lib.rs         # #![forbid(unsafe_code)] #![deny(missing_docs)]
│   │   ├── src/layer/         # LayerName, LayerKind, Visibility, LayerMeta, Layer
│   │   ├── src/stack/         # Source, Ref, Stack, ResolvedStack (typestate)
│   │   ├── src/item/          # Agent, Skill, Command, Rule, Hook, Permission, McpServer, Model
│   │   ├── src/merge/         # one module per item type; the merge algebra (§4)
│   │   ├── src/target/        # Target trait (sealed); claude/, later codex/: anti-corruption layer
│   │   ├── src/render/        # FileTree (in memory), deterministic serialisation
│   │   ├── src/lock/          # Lock, ObjectId, ContentHash
│   │   ├── src/identity/      # CommitIdentity and its decision rules
│   │   ├── src/account/       # AccountName, declarations, resolution decision (pure)
│   │   └── src/diagnostic.rs  # Diagnostic { code, message, at }, no printing
│   ├── alz-io/                # library: every effect, behind capability types
│   │   ├── src/fs/            # ReadOnlyDir, RepoRoot (write), CacheDir, StateDir, safe walks (openat via
│   │   │                      # rustix >= 1.1.6, decision-27), O_NOFOLLOW reads, atomic write + rename, locks
│   │   ├── src/git.rs         # GitRunner: the git binary, argument arrays, explicit environment,
│   │   │                      # no shell, no git library (decision-24)
│   │   ├── src/cache.rs       # cache layout and integrity, on top of fs
│   │   ├── src/terminal.rs    # PersonAtTerminal proof value (only constructible from a real TTY
│   │   │                      # with no agent marker)
│   │   └── src/env.rs         # Env snapshot taken once at start; presence-only queries
│   ├── alz/                   # binary: clap CLI, wiring, rendering diagnostics, exit codes
│   │   ├── clippy.toml        # disallowed-methods: std::fs, std::process, std::env (decision-23)
│   │   ├── src/main.rs        # a few lines: try_parse, run, map Result to ExitCode
│   │   ├── src/cli.rs         # clap derive types; clap errors rendered and sanitised by alz (decision-26)
│   │   ├── src/command/       # one module per command; each is a thin use case
│   │   └── tests/             # assert_cmd + snapshot tests; files set up through alz-test-support
│   └── alz-test-support/      # publish = false; isolated HOME, fixture copies, git helpers
├── xtask/                     # later, added by the first task that needs it: parity run, release prep
├── fuzz/                      # later, added by the first task that needs it: cargo-fuzz targets for the parsers
├── test/fixtures/             # shared by both implementations until R8, then moved under crates/alz
└── src/, test/, package.json  # TypeScript reference, removed at cutover
```

Conventions inside the tree (from `gr-rust`): folders such as `layer/` sit beside a `layer.rs`
that declares them (no `mod.rs`); every crate inherits `[workspace.lints]`; items are re-exported
one by one, with no glob and no prelude.

Why three crates (core, io, binary) and not one, and not ten. The usual advice for a small CLI is
to start with one crate and split only for a reason; alz has one reason that the compiler can
enforce:

- **`alz-core` cannot do I/O, enforced by the compiler**: it does not depend on `alz-io`, and its
  `clippy.toml` lists `std::fs`, `std::process` and `std::env` under `disallowed-methods`. That
  turns "the merge is pure" from a convention into a build error.
- **`alz-io` owns every effect** and exposes them as capabilities (§3.4), so the safety rules live
  in one place a security reviewer can read end to end.
- **`alz` is thin**: argument parsing and wiring only. Its own `clippy.toml` bans the same
  `std::fs`, `std::process` and `std::env` functions (decision-23), so "only `alz-io` performs
  effects" is a build error on both sides, not a convention. clap still reads the arguments.
- More crates would follow folders, not boundaries (a TypeScript habit); crates cost build time
  and API ceremony. Modules give enough structure inside a crate. Revisit only if build times or
  a real second consumer (an editor extension, a library user) appear.

## 3. Type-level design applied to alz

The aim: make invalid states unrepresentable, so most rules are checked by `rustc` and only the
rest by tests. Count states: if a type can hold values the spec forbids, the difference is bugs
waiting for a test.

### 3.1 Parse, don't validate: newtypes with private fields

Every value from untrusted input becomes a type with one smart constructor that returns `Result`:
`LayerName`, `AccountName` (`^[a-z0-9][a-z0-9-]*$`, at most 64), `SafeRelPath` (no `..`, no
absolute, no backslash, ASCII where the spec says so), `ObjectId` (an enum `Sha1([u8; 20]) |
Sha256([u8; 32])`, not a string), `ContentHash`, `GitRef`, `EnvVarName`. Downstream code takes the
type, never `&str`, so a check cannot be forgotten.

### 3.2 Sum types instead of flags and strings

- `Source = Path { dir, ref_: Option<GitRef> } | Git { url, ref_ } | GitHub { owner, repo, ref_ }`.
- `Visibility = Public | Private | Confidential` with `Ord` (privacy rank is the order).
- Account outcomes: `enum Applied { Folder(AccountFolder), Default, Kept(Foreign) }` and
  `enum FailsClosed { MissingFolder, CacheMiss { recorded }, Changed, SourceGone, Conflict,
  UnreadableManifest, ConfiguredFolder, InvalidName, RelativeHome }`, matched exhaustively, so a
  new reason is a compile error everywhere it must be handled, not a missed `if`.

### 3.3 Typestate for the pipeline

`Stack<Declared>` → `Stack<Resolved>` → `Merged` → `Rendered` → `Written`. Each step consumes the
previous state and returns the next, so rendering an unmerged stack or writing an unchecked tree
does not compile. Use separate types where states carry different data; a `PhantomData` marker
only where they carry the same data.

### 3.4 Capabilities: effects you can only reach with a token

- `ReadOnlyDir` has read methods and nothing else. Account resolution takes `&ReadOnlyDir` and
  no `GitRunner`, so "resolution never writes and never runs git" holds by construction; the
  tracing tests of the TypeScript suite become a type signature plus one small test.
- `RepoRoot` (write inside the repository only) and `StateDir` (write `allowed.yaml` only) are
  distinct types; `insideRoot` becomes "a `RepoRoot` can only produce paths below itself".
- `PersonAtTerminal` is a proof value: constructible only by the terminal module after `isatty`
  and the agent-marker check, and required by `account allow`. No code path can skip the gate.

### 3.5 Merge rules as algebra

The merge rules are small algebraic structures, and naming them makes the properties testable:

- "Most specific layer wins" for scalars is the **last-writer-wins monoid**: associative, not
  commutative; identity is "not set".
- Collections that combine across layers are **set or map unions**: associative, commutative,
  idempotent (a join-semilattice).
- The **security floor** (deny rules, locked hooks, locked reviewers, the account allow gate) is a
  **lattice with a monotone merge**: a later layer can move a setting up (stricter) but never down.
  `allow ⊑ ask ⊑ deny` is the order; merge is the join.
- A build is a **fold** of the stack with these merges, then a pure render, so determinism follows
  from purity plus ordered collections (`BTreeMap`, never `HashMap` in output paths).

Property tests (`proptest`) state these laws directly: associativity of every merge, idempotence
of unions, monotonicity of the security floor, and "build(x) == build(x)" byte for byte.

### 3.6 Totality and panics

Library functions are total: every input yields `Ok` or a typed `Err`. No `unwrap`, `expect`,
indexing that can panic, or `unreachable!` outside tests, enforced by clippy restriction lints.

## 4. Is DDD applicable? A critical assessment

DDD was made for large business systems: many people, a changing business, persistent entities,
transactions and several teams with different models. alz has almost none of that. Applied
wholesale it would add ceremony and TypeScript-era (and Java-era) patterns that decision-22 rules
out. Some parts fit very well, though.

| DDD idea | Fit for alz | Use it as |
| --- | --- | --- |
| Ubiquitous language | **Strong.** The spec already defines the vocabulary: layer, stack, overlay, slot, lock, declaration, allow, fail closed. | Type and module names are the spec's words, exactly. No synonyms. |
| Value objects | **Strong.** Almost everything is a value with invariants. | Newtypes and enums (§3.1, §3.2). |
| Anti-corruption layer | **Strong.** Claude Code and Codex have their own formats and quirks (target-facts). | The `target` module translates the neutral model; native quirks never reach `merge`. |
| Subdomains | **Moderate.** Layer composition is the core; accounts and commit identity are supporting subdomains with their own words. | Modules inside `alz-core`, not separate crates or services. |
| Bounded contexts and context maps | **Weak.** One model, one team, one binary. | Not used. The target adapters are the only translation boundary. |
| Entities | **Weak.** Few things have identity over time; a layer is identified by its source and revision, which is a value. | Not modelled as entities. |
| Aggregates | **Weak.** No concurrent mutation or transactions, except `allowed.yaml`. | `allowed.yaml` is the one consistency boundary: one writer under a lock, invariants checked before write. Not a general pattern. |
| Repositories, services, factories | **Poor.** Interfaces over storage are a mocking habit; Rust has smart constructors and capabilities. | Not used. Plain functions over capability types. |
| Domain events | **Poor.** Nothing subscribes; output is files. | Not used. |
| Layered architecture (domain / application / infrastructure) | **Partly.** The useful part is "domain has no I/O". | Enforced by the crate split (§2), without extra layers. |

**Verdict.** Use DDD's *language* discipline and *value objects*, and the anti-corruption layer
for targets; drop aggregates, repositories, services and events. The design discipline that fits
alz better is type-driven design (§3) plus algebraic merge laws, inside a functional core with an
imperative shell. The personal Rust skill should say when DDD helps (business systems with real
entities and transactions) and when it does not (compilers and CLIs like alz).
