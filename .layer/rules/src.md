---
paths:
  - "src/**"
---
- `src/` is the frozen TypeScript reference and is never changed, not even for a bug or a
  security flaw (decision-30). A bug found here is fixed only in the Rust `alz` under `crates/`,
  and its acceptance case is marked reference drift. The rules below describe the existing code.
- Relative imports without a `.js` extension. Named exports; no default exports.
- Untrusted input (repo manifest, lock, layer files, remote layers) is validated where it is
  read. Paths pass `isSafeRelativePath`; every write and delete goes through `insideRoot`.
- Subprocesses take an argument array. Values from untrusted input are validated or placed
  after `--`.
- User mistakes become `diags.error(code, message, where)` so every problem is reported in one
  run. `FatalError` only when the build cannot continue.
- Output is deterministic: `stableJson`, `stringifyYaml`, sorted lists, no timestamps.
- New runtime dependencies need a recorded decision (`backlog decision create`).
