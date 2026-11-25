---
id: decision-40
title: >-
  Adapters are pure functions behind a versioned neutral model; built-in
  adapters are Rust crates on that contract, targets are open identifiers,
  third-party adapters run later as sandboxed WASM components (proposed,
  TASK-55)
date: '2026-11-25T19:01:00Z'
status: proposed
---
## Context

The owner's direction, relayed by the coordinator on 2026-11-26 for review (so this record is `proposed` until the owner accepts it): the first release supports Claude Code, and later any open-source developer can add an adapter for Codex, Google's Antigravity or another agent tool without forking alz.

What the code does now (branch base 1d49603):
- `crates/alz-core/src/target.rs` holds `TargetName`, a closed enum `{Claude, Codex}` with `ALL` and the error text "must be a target: claude or codex". About 40 files use it: the manifest's `targets`, the target sections of `settings.yaml` and `models.yaml`, an agent's `claude:` and `codex:` fields, native hook events, `targets/<target>/` passthrough, the native key table, the merge, the render dispatch, the live probe, and `alz-io`'s account listing, which uses it for the `.claude` and `.codex` account-folder families.
- The Claude adapter (`crates/alz-core/src/target/claude/`) is already pure. `ClaudeFiles::of(&Merged)` and `ClaudeSettingsFiles::of(&Merged, &Workspace)` return a `FileTree` and `Diagnostics`, and nothing is written. But it reads alz-core's internal `Merged` directly, so every change to the merge types reaches it.
- `crates/alz-core/src/render/rendered.rs` chooses the adapter with a `match` on `TargetName`. The `Codex` branch is the error "the 'codex' adapter is not implemented yet". Claude knowledge also sits in the core: `CLAUDE_MANAGED_FOLDERS`, `CLAUDE_MEMORY_PATTERNS` and `claude_md` (rendered in `render/instructions`, not in the adapter).
- `crates/alz-core/src/item/native_keys.rs` holds the per-target table of first-class, reserved and floor keys, with rows for both targets.
- Spec §8 already says "Adapters are plugins of the tool (one module each), so new targets need no core changes", but the code does not make that true. doc-6 §2 plans a sealed `Target` trait, and decision-23 fixes the workspace at four crates.
- "An adapter relies only on what `docs/target-facts.md` records" is enforced for Claude only, by `crates/alz-core/src/target/claude/tests/facts.rs`. That test renders a stack that writes every key kind of §8.1 and reads the keys back with Claude-specific extractors. It looks each key up in the target-facts sections the spec names, and ratchets with `NO_FACT_RECORDS`.

Options for third-party adapters:
- **Plugin executables over JSON on stdio**: rejected. Arbitrary code would run with the user's privileges and could read `.env*` files, account folders and the network, which §9.1 forbids alz itself. Its output could depend on the clock or the environment, which breaks the deterministic build.
- **Dynamic libraries (`dlopen`)**: rejected. They are `unsafe`, Rust has no stable ABI, and they run with the same privileges as executables.
- **A declarative mapping file (templates)**: not chosen. It is too weak for real tools, whose tool-name tables, model checks and conditional keys need logic.
- **WASM components in a host with no imports**: chosen for third parties. They can be written in any language, they have no file system, network or clock, and they are deterministic by construction.

## Decision

1. **Ports and adapters.** The core publishes a **neutral model**: the merged layers for one target, versioned (`major.minor`, starting at 1.0) and documented. An adapter is a **pure function**, `render(model, context) -> output`, where the output is the generated files and the diagnostics. An adapter never touches the file system, a process, the environment or a clock. The core's writer applies every rule of §4.3, §4.5 and §9, so an adapter cannot weaken them.
2. **Contract crate.** A new crate, `alz-adapter`, holds `TargetId`, the model v1 types, `Descriptor`, `Context`, `Output` and the `Adapter` trait.
   - It depends on no other alz crate. Its `clippy.toml` bans `std::fs`, `std::process` and `std::env`, as alz-core's does.
   - alz-core depends on it and converts its internal `Merged` into the model. This is an anti-corruption layer in the other direction, so the merge types can change without breaking adapters.
   - The trait is public, not sealed, which replaces doc-6 §2's "Target trait (sealed)". The binary decides which adapters it registers.
3. **What an adapter states and returns.**
   - The **descriptor** is static data:
     - the id, and the model version it supports;
     - the verified range of the tool's versions, for `alz doctor`;
     - its territory: the files and folders it may write;
     - its managed folders, protected patterns and always-adopted files;
     - its native vocabulary: settings keys with types, and the first-class, reserved and floor keys with their order;
     - its native agent fields and native hook events;
     - its facts file.
   - The **output** is files and diagnostics:
     - A file is either structured content (`Json`, `Toml`, or `Markdown` with optional YAML frontmatter) or `Bytes` with an executable bit.
     - The core emits structured content with its own stable emitters and inserts the generated header (§4.3). So the header and the key order cannot be wrong, and the facts check can list the written keys without a parser for each adapter.
     - Diagnostic codes are either the shared adapter codes the spec lists or `<id>-<word>`.
     - Messages are untrusted text and are stripped.
4. **The core checks every output before the writer**, and any failure is the error `target`.
   - It refuses:
     - a path that is not a `SafeRelPath`;
     - a path outside the territory;
     - a core-owned path: `AGENTS.md`, `layer.yaml`, `layer.lock`, `.layer/**` or `.git/**`;
     - output past the file-count and size limits;
     - a code outside the adapter's namespace.
   - Protected paths are dropped as today.
   - The native settings of a target are merged by the core with the descriptor's key rules, so the security floor (§7.6, §7.9, §9.3) stays core law.
   - The core also requires a **coverage report**. Every security item of the model (each deny rule, each floor field, the autonomy level of doc-7) is listed as expressed, approximated or not expressed, so an adapter cannot drop a deny rule silently. Whether "not expressed" is a warning or an error is an owner question, answered in TASK-104 before the spec states it.
5. **Open target identifier.** `TargetName` becomes `TargetId`, a validated newtype: `^[a-z][a-z0-9-]{0,31}$`.
   - A well-formed id without a registered adapter is the error `target`: "no adapter for '<id>' (available: claude)". `codex` keeps its current "not implemented yet" message until its adapter exists.
   - Native sections for an id that has no adapter and is not in `targets` are kept, with their shape checked, and are never written.
   - The `.claude` and `.codex` account-folder families of §7.13 and §9.1 get their own closed type, because accounts are not adapters.
6. **Built-in adapters are crates**: `alz-target-claude` now, `alz-target-codex` later. Each depends only on `alz-adapter`, never on `alz-core` or `alz-io`. So "an adapter sees only the model and cannot do I/O" is a build error.
7. **Per-adapter verified facts.** The rule "rely only on recorded facts" applies to every adapter, and its test becomes a reusable harness.
   - The harness renders a coverage stack and lists every written key from the structured output.
   - It looks each key up in the adapter's facts file, under the sections the adapter maps each key kind to.
   - Each adapter keeps its own `NO_FACT_RECORDS` ratchet.
   - Built-in adapters must pass it in CI. Third-party authors get it in a conformance check (TASK-125).
8. **Third-party adapters come in a later step of milestone m-8.**
   - They are WASM components: a WIT world `alz:adapter@1` that exports `describe` and `render` and imports nothing.
   - The host refuses any component that imports anything, and it limits fuel or epochs and memory.
   - They are declared in the repository manifest and pinned with a content hash in `layer.lock`. That part is still an owner question (TASK-124).
   - The WASM runtime is **not** chosen here. wasmtime is a large dependency and needs its own intake decision (TASK-121), in the style of decision-25 and decision-38: proposed by the architect, accepted only by the owner.

## Consequences

- **Amends decision-23**: the workspace gains `alz-adapter` and `alz-target-claude`, and later `alz-target-codex` and a WASM host crate. doc-6 §2 (the sealed trait) and the anti-corruption row of doc-6 §4 are updated by TASK-104 once this is accepted.
- **What moves**:
  - `TargetName` and `TargetError` become `alz-adapter`'s `TargetId` (TASK-105, TASK-106).
  - The Claude adapter reads the model, not `Merged` (TASK-107).
  - `render/rendered.rs::adapter` becomes a registry lookup. `CLAUDE_MANAGED_FOLDERS`, `CLAUDE_MEMORY_PATTERNS` and `claude_md` move into the Claude descriptor and adapter (TASK-108).
  - The output checks and the coverage report are added (TASK-109).
  - The Claude and Codex rows of `item/native_keys.rs` move into descriptors. The core keeps `Strictness` and the floor check (TASK-110).
  - The crate move (TASK-111).
  - `target/claude/tests/facts.rs` becomes the harness plus Claude's subject map (TASK-112).
  - `alz-io/src/account/listing.rs` matches its own account-family type.
- **User-facing**:
  - Spec §5.2: `targets` accepts any well-formed id.
  - Spec §8 gains the "no adapter for" message.
  - A new §8.0 states the contract and the model version, and §10.1 the adapter codes and the `target` checks.
  - Generated output for `targets: [claude]` stays byte for byte the same; the acceptance suite and `mise run alz:check` prove it after each task.
  - The wider grammar and the new message need a minor version bump and a CHANGELOG entry.
- **Model versioning**: a minor version only adds.
  - In Rust, the model structs are `#[non_exhaustive]` with accessors.
  - In WIT, a minor version adds functions or interfaces, because WIT records cannot grow.
  - A different major version is refused at registration: a compile error for built-in adapters, and the error `target` for components.
- **Cost**: one large but mechanical type migration, and a crate move that must give byte-identical output. Until the WASM step only built-in adapters exist, so the open id changes error messages and nothing else.
- **Risk**: an adapter can still map a neutral item wrongly. The core cannot check what a native key means, only where files go, their shape, the floors and the coverage.
  - For built-in adapters, the facts harness and review cover this.
  - For third-party adapters, the user who names the target trusts the adapter's mapping but never its access, because it has none.
- Related: TASK-18 (Codex adapter) is built on this contract by TASK-126. doc-7 (the autonomy proposal) relies on the coverage report and on each descriptor's mapping.
