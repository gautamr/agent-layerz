---
paths:
  - "crates/**"
  - "xtask/**"
  - "fuzz/**"
  - "Cargo.toml"
  - "Cargo.lock"
  - "deny.toml"
  - "clippy.toml"
  - "mise.toml"
  - "mise.lock"
---
- Follow the gr-rust skill. The TypeScript in `src/` describes behaviour only; never copy its
  structure, types, errors or names (decision-22).
- `alz-core` has no I/O: no `std::fs`, `std::process` or `std::env` (its `clippy.toml` bans
  them; `crates/alz/clippy.toml` is its twin, and the two change together). Effects live in
  `alz-io` behind capability types; the `alz` binary only wires (doc-6).
- Repository content, `layer.lock` and remote layers are untrusted: parse them into validated
  types at the boundary. Writes happen only through the repository-root and state-directory
  capabilities.
- Generated files are byte-identical to the TypeScript reference for every fixture until
  cutover, except where the spec and a reference-drift case say otherwise (decision-30); output uses ordered collections, never `HashMap`, and no timestamps.
- Every new dependency is a recorded decision. No `unsafe`, no `allow` without
  `#[expect(…, reason = "…")]`.
- Tests use an isolated `HOME` and temporary directories; never the real `~/.claude*`,
  `~/.codex*` or `~/.cache`.
