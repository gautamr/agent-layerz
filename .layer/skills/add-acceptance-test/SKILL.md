---
name: add-acceptance-test
description: Write an agent-layerz acceptance test with its fixture. Use when adding or changing CLI behaviour, merge rules, adapters, adoption or a security check, and for every bug fix's regression test. Not for pure helper functions, which may use a small unit test.
---
# Add an acceptance test

Acceptance tests run the real binary as a child process against a copied fixture. They are
the proof that a behaviour or guarantee holds, and they are language-neutral: the same suite
checks the TypeScript reference and the Rust `alz`.

## Which binary
- `ALZ_BIN` is an absolute path to an executable file; it is checked before any test runs.
  When unset, the suite bundles the TypeScript reference into the run root and runs that.
- `mise run test:acceptance` runs only the acceptance project and honours `ALZ_BIN`, e.g.
  `ALZ_BIN=$PWD/target/release/alz mise run test:acceptance`.
- One `alz-suite-*` run root is made per run and removed at teardown (`test/setup/run-root.ts`).

## Layout
- Test: `test/acceptance/NN-<topic>.test.ts`. Use the next free `NN`.
- Fixture: `fixtures/NN-<topic>/`, usually `personal/` (a layer with `layer.yaml`) and
  `repo/` (a `layer.yaml` stack plus `.layer/`). Add more layers or repos as the case needs.
- Expected output, when comparing whole files: `fixtures/NN-<topic>/expected/<path>`,
  read with `expected(name, path)`.
- A case that only the TypeScript code can show (in-process runs, fake terminals, source
  scans, resolver internals, the install script) goes in `test/reference/`, not here. Those
  files are deleted at cutover.
- The spec's formulas (cache names, content hash, limits) are test-side code in `test/spec/`.
- The suite asserts the spec, not the TypeScript reference, which is never changed
  (decision-30). A case the reference is known to fail is written with
  `referenceDrift("TASK-n", name, fn)` from `test/drift.ts`: it must fail against the bundled
  reference and pass against any ALZ_BIN, and it names the task that records the drift.

## Helpers (`test/helpers.ts`)
- `fixture(name)` copies the fixture to a temp dir and gives it an isolated `HOME`, so tests
  never see the real `~/.claude`.
- `layers(ws, dir, args, env)` spawns the binary under test with `<args>` in `ws.root/dir` and
  returns `{ code, stdout, stderr }`. `ALZ_CLAUDE_BIN` defaults to a missing binary; point it at a
  fake script in the fixture to test `ls --live` or `doctor`.
- `readFile`, `exists`, `readTree` (whole directory as path → content, for "nothing changed"),
  `snapshot` (the same with kind, mode, link target and unreadable files, links not followed).
- `test/account-workspace.ts` sets up account-layer cases. Child processes go through
  `test/exec.ts` (`tool`, `capture`, `spawnCaptured`: own process group, 15 s timeout).
- Temp folders come from `tempDir()` in `test/run-root.ts`, never from `tmpdir`.
- Acceptance tests never import `src/`, directly or through a helper. The guard
  `test/guards/acceptance-imports.test.ts` fails if one does.

## Rules
- Name the test after the guarantee, not the implementation: "refuses a lock that lists a
  path outside the repo", not "validates lock paths".
- Assert the exit code, the message the user sees, and the files on disk. The assertions follow
  `docs/LAYERING-SPEC.md`, not the TypeScript wording.
- Help, usage errors and version follow decision-26: `--version` exits 0 with empty stderr and
  stdout that contains the version; `--help` exits 0 with stdout not empty; a usage error exits
  2 with empty stdout and stderr not empty. Never compare their full text.
- For a guarantee that something must not happen, compare before and after: `readTree` (paths
  and text) for files alone; `snapshot(dir)` (kind, mode, link target, and files that cannot be
  read) when a link, a mode or an unreadable file could change; `gitProxy()`
  (`test/git-proxy.ts`) for "git is never run", which sees only a `git` found on PATH.
- Hooks and commands that contain `${CLAUDE_PROJECT_DIR}` are literal strings: mark them with
  `// biome-ignore lint/suspicious/noTemplateCurlyInString: Claude expands this literal itself.`
- A test that needs a real Claude login is skipped unless `ALZ_SMOKE_CLAUDE=1`.
- Run the single file first: `npx vitest run test/acceptance/NN-<topic>.test.ts`, then
  `mise run test:acceptance` (also with `ALZ_BIN` set once a Rust binary exists), then
  `mise run test`.
