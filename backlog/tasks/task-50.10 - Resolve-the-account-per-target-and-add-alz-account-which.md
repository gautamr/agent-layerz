---
id: TASK-50.10
title: Resolve the account per target and add alz account which
status: Done
assignee:
  - '@claude'
created_date: '2026-11-22T03:33:00Z'
updated_date: '2026-11-22T09:09:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.9
references:
  - docs/LAYERING-SPEC.md
  - docs/target-facts.md
parent_task_id: TASK-50
priority: high
type: feature
ordinal: 27390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Third slice of TASK-50.2. The resolver and the read-only alz account which: the resolution order (an alz use choice arrives as an argument, LAYERING-SPEC §7.13 'alz use stays in the shell'), the user config and the allow state read with their safety checks, path matching and the root walk, the environment source (ALZ_SET_ markers), fail closed and what the hook would apply. Step 4 uses the previous slice's cache reader. Tests write allowed.yaml themselves; alz account allow comes in the next slice.

This is the resolver that alz run (TASK-50.3), alz use and the hook (TASK-50.6) and alz doctor (TASK-50.4) will call, so its result per target says what applies, why each step was skipped, and, when a named account fails closed, what the hook sets and which fix to print.

Out of scope: writing allowed.yaml, the allow hints in build and check, alz run, the hook.

Spec: LAYERING-SPEC §7.13 'User config', 'Allow state', 'Both files', 'Resolution', 'Paths', 'No fetch on resolve', 'A named account that cannot apply fails closed', 'Values set outside alz', §9.1, §10 'alz account which', §13 tests 19, 20 and 24. Facts: docs/target-facts.md §12 items 10, 11, 18 and 20.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 alz account which prints the resolved account per target, its source (use, environment, user config, layer or default; use comes from the alz shell function, environment from comparing the variable with its ALZ_SET_ marker), the reason for each skipped step and the paths of accounts.yaml and allowed.yaml; when a named account fails closed it prints 'layer step skipped (<reason>); fails closed', what the hook applies (the missing folder, the recorded account or the blocked folder) and the fix command; tests cover each source, each fail-closed case and a missing default folder (never fails)
- [x] #2 The user config (accounts.yaml, hand-written, never written by alz) and the allow state (allowed.yaml) live outside every repository; alz ignores either with a warning when its real path is inside the current repository, the user does not own it, group or others can write it, its directory or a parent directory below $HOME, or the XDG_ variable that locates it points somewhere other than its default (~/.config or ~/.local/state, compared as real paths) while __MISE_DIFF or DIRENV_DIFF exists, the warning then naming the variable and saying that mise or direnv is active here; alz checks only that those two variables exist and never reads, decodes, prints or logs their values (decision-17; test: a marker string put in them appears in no output); an allowed.yaml entry with a relative root or an invalid account name is ignored with a warning; and alz build never writes an account value into a generated file
- [x] #3 Resolution (shell hook, alz run, alz account which) never fetches, clones or runs git and writes nothing: for a root not in allowed.yaml it reads no file of the stack, and otherwise it reads only layers it can locate without git (see the cache index criterion), with build's validation, and checks each against the lock's content hash (TASK-51), a mismatch counting as a cache miss; a cache miss skips the layer step, and in an allowed repository that fails closed as in TASK-50.3 and TASK-50.6 (tests trace git calls and file reads)
- [x] #4 Paths resolve with the file system's spelling: a folders entry matches whole path segments and the longest wins, an allowed entry matches only the exact real root, the root is the nearest layer.yaml with a stack key at or above the working directory (one without it is skipped), each layer.yaml read is parsed up to 1 MiB with a YAML alias limit, and an entry that does not resolve matches nothing (tests: symlink, case variant, nested manifest, a layer manifest on the way)
- [x] #5 An account named by a layer takes effect only while allowed.yaml holds an entry for the repository's real root whose whole per-target map and declaring layer's source (a remote URL without its ref, or a local layer's real path) equal the resolved ones; any difference skips the layer step for every target, while a ref bump does not; alz account which prints the alz account allow command only for a changed entry and otherwise reports only that the layer step was skipped; accounts set in the user config need no approval (tests with allowed.yaml written by the test: no entry gives the default account, a matching entry the named one, a changed map or source skips it, a ref bump keeps it)
- [x] #6 A declaration by a layer that may not declare is ignored with one warning by alz account which, in the resolver that alz run and the hook will use (LAYERING-SPEC §7.13 'Who may declare it', §13 test 24, in test/acceptance/24-untrusted-repo.test.ts)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## 0. Scope and contract

- Delivers the account resolver that alz run (TASK-50.3), alz use and the hook (TASK-50.6) and alz doctor (TASK-50.4) will call, and the read-only `alz account which`. Out of scope: writing allowed.yaml and the allow hints (TASK-50.11), alz run, the hook, doctor, TASK-53 (build's cacheRoot with an empty or relative XDG_CACHE_HOME), TASK-52 (resolving repository paths one component at a time).
- Resolution writes nothing, runs no git and fetches nothing. It reads only: the layer.yaml files on the root walk, accounts.yaml, allowed.yaml, the $HOME listing (names only, §9.1), and, only for an allowed root, what readAccountDeclaration (TASK-50.9) reads.
- User-facing: a new command (`alz account which`) and two new files that alz reads. The layer format and generated output do not change. The commit is `feat(accounts): ... (task-50.10)`, and the change ships in the TASK-50 minor release. README "Commands" gains the which row.
- The two owner decisions in the notes (the root walk, and XDG_CACHE_HOME under mise or direnv) go into the spec in the same PR (section 9).

## 1. Modules

All new modules go in src/resolve/ with an account- prefix, so the map in AGENTS.md stays true and .layer/ does not change. None of them imports resolve/git.ts, tree-cache.ts, sources.ts, pipeline.ts or node:child_process. `import type` lines are erased and do not count.

| File | Change |
|---|---|
| src/core/files.ts | `RegularFile` gains `uid` and `mode` from the same fstat (existing callers unchanged) |
| src/core/env.ts | `envManagerActive(env)`: `Object.hasOwn` plus `typeof === "string"` for `__MISE_DIFF` and `DIRENV_DIFF`; never `env[name]` for them |
| src/core/xdg.ts (new) | `xdgBase(env, name, home, fallback)` gives `{ dir, fromVariable, relativeIgnored }`. An empty or relative value counts as unset (XDG Base Directory). TASK-53 later moves cacheRoot onto it |
| src/core/sanitize.ts | `displayPath(path, home)`: home shown as `~`, control and format characters removed. `sanitizeOrigin` may call it |
| src/resolve/account-folders.ts | `accountFolder(home, target, name)`, `blockedFolder(home, target)`, and `listAccountNames(home)`: one `readdirSync(home, {withFileTypes})`. A directory or a symlink named `.claude-<n>` or `.codex-<n>` counts as existing; it is never stat'd. `alz-blocked` is never counted |
| src/resolve/account-root.ts (new) | The root walk (section 3) |
| src/resolve/account-files.ts (new) | Locating both files, the safety checks and the bounded read (section 4) |
| src/resolve/account-user-config.ts (new) | Parsing accounts.yaml and matching folders (section 5) |
| src/resolve/account-allow-state.ts (new) | Parsing allowed.yaml, finding the entry for a root, `sameMap` (section 6) |
| src/resolve/account-environment.ts (new) | Foreign value detection per target (section 7, step E) |
| src/resolve/account-resolution.ts (new) | `resolveAccounts` and its types (sections 2 and 7) |
| src/resolve/account-messages.ts (new) | Fixed phrases: skip reasons, miss reasons, fail-closed reasons, fixes, what the hook applies. Shared by which, run, the hook and doctor |
| src/cli/commands/account.ts (new) | `accountWhichCommand(io, { use })`. TASK-50.11 and TASK-50.5 add allow and init here |
| src/cli/main.ts | `account` routes its subcommands; USAGE gains `account which` |

## 2. Resolver API

```ts
export interface AccountInput {
  cwd: string;                 // the working directory, as given
  home: string;                // $HOME; folders are built from it as given
  env: Env;                    // read by name only (list below); never spread or copied
  uid: number | undefined;     // process.getuid?.(); undefined: both files are ignored
  targets?: AccountTarget[];   // default: claude, codex
  run?: { target: AccountTarget; account: string }; // step 1, alz run --account (TASK-50.3)
  use?: AccountMap;            // step 2, passed by the alz shell function (TASK-50.6)
}
export function resolveAccounts(input: AccountInput): AccountResolution; // never throws for content problems

export interface AccountResolution {
  repository: { status: "found"; root: string; manifest: "read" | "unreadable" | "invalid" }
            | { status: "none"; reason: "not-found" | "account-folder" | "cwd-unresolvable" };
  accountsFile: AccountFile;   // { path as located; status: read | missing | ignored; reason? }
  allowedFile: AccountFile & { allowed: boolean };
  targets: TargetAccount[];
  warnings: { code: string; text: string }[]; // text already sanitised; fixed order; each once
}
export interface TargetAccount {
  target: AccountTarget; variable: "CLAUDE_CONFIG_DIR" | "CODEX_HOME";
  source: "run" | "use" | "environment" | "user config" | "layer" | "default";
  account?: string;            // a validated name or "default"; undefined for a foreign value
  apply: { action: "set"; folder: string } | { action: "unset" } | { action: "keep" };
  skipped: { step: "use" | "user config" | "layer"; reason: SkipReason }[]; // consulted, named nothing
  failsClosed?: FailClosed;    // run and use stop; the hook warns and applies `apply`
}
type FailClosed =
  | { kind: "missing-folder"; account: string; existing: string[] } // apply: the missing folder; fix: alz account init <name>
  | { kind: "cache-miss"; miss: MissReason | "cache-location"; entry?: { index: number; source: string };
      recorded: string; folderMissing?: true }                        // apply: the recorded account; fix: alz build
  | { kind: "changed"; allowed: AllowedEntry; now?: { account: AccountMap; source: string } } // apply: blocked; fix: alz account allow, or allow --remove when `now` is absent
  | { kind: "source-gone"; allowed: AllowedEntry }                    // apply: blocked; fix: alz account allow
  | { kind: "conflict"; clients: LayerRef[] }                         // apply: blocked; fix: remove one declaration, then alz build
  | { kind: "manifest"; problem: "unreadable" | "invalid" };          // apply: blocked; fix: repair <root>/layer.yaml
```

- Environment keys read: values of CLAUDE_CONFIG_DIR, CODEX_HOME, ALZ_SET_CLAUDE_CONFIG_DIR, ALZ_SET_CODEX_HOME, XDG_CONFIG_HOME, XDG_STATE_HOME and XDG_CACHE_HOME; only the presence of `__MISE_DIFF` and `DIRENV_DIFF`. Nothing else is read, ALZ_USE_* included.
- The result holds no file content and no environment value other than the account variables, which go through displayPath before printing. Sources and layer names stay raw, and callers print them through `quote`.

## 3. Root walk (account-root.ts)

1. If `namesAccountFolder(cwd as given)`, there is no root (`account-folder`) and no file system call is made. If realpath of cwd fails: `cwd-unresolvable`. If `touchesAccountFolder(real cwd)`: `account-folder`.
2. For each dir from the real cwd upwards:
   - If `touchesAccountFolder(dir)` ($HOME or above, inside an account folder, or the folders the variables name), stop with no root (`not-found`). The walk never opens a layer.yaml there.
   - Otherwise read `dir/layer.yaml` with `readRegularFile(path, MAX_LAYER_YAML_BYTES)` (1 MiB, O_NOFOLLOW).
     - `missing` (ENOENT or ENOTDIR): go to the parent.
     - `invalid` (a link, a folder, over 1 MiB, permission denied): root = dir, manifest `unreadable`.
     - `parseYamlLimited` throws (invalid YAML, or past the alias limit): root = dir, `unreadable`.
     - No `stack` key (isStackManifest): go to the parent.
     - Otherwise `parseManifest`. If it passes: root = dir, `read`, with the stack kept. An InputError: root = dir, `invalid`.
   - Reaching `/`: `not-found`.
3. An unreadable or invalid manifest gives one warning naming its path and the problem as a fixed phrase, without file content. The walk never skips past it (owner, 2026-11-22). The root's manifest is read once, and step 4 reuses the parsed stack.

## 4. Both files (account-files.ts)

- Location: `xdgBase(XDG_CONFIG_HOME, ~/.config)/agent-layerz/accounts.yaml` and `xdgBase(XDG_STATE_HOME, ~/.local/state)/agent-layerz/allowed.yaml`. A relative value is ignored with a warning; an empty value is ignored without one.
- Checks, in order. The first that fails sets `ignored`, gives one warning naming the file (displayPath) and the reason, and nothing further is opened:
  a. `namesAccountFolder(path as located)`: ignored before any file system call.
  b. `realpath.native(path)`: ENOENT gives `missing` (no warning); any other error gives ignored.
  c. `envManagerActive(env)`, the variable set to an absolute path, and `realOrResolved(value) ≠ realOrResolved(home/default)`: ignored. The warning names the variable and says "mise or direnv is active here".
  d. `uid` undefined: ignored ("cannot check the owner here").
  e. `touchesAccountFolder(real)`: ignored.
  f. A root was found and `isWithin(root, real)`: ignored ("inside the current repository").
  g. `readRegularFileWithId(real, 1 MiB)`: `missing` gives missing; `invalid` gives ignored.
  h. fstat uid ≠ uid: ignored ("not owned by you"). `mode & 0o022`: ignored ("group or others can write it").
  i. Two directory chains: `dirname(real)`, and the real path of the folder that holds the name as located (the link's folder when the file is a link). For each, lstat that directory and every parent strictly below the real $HOME (only the directory itself when it lies outside $HOME). Any `mode & 0o022`: ignored, naming the directory.
  j. `parseYamlLimited` throws: ignored ("not valid YAML").
  k. A warning only; the file is still read: lstat `<ancestor>/.git` for each ancestor of the real path, up to `/`. A hit means it lies inside a git work tree. No git runs.
- An agent that runs as the user can still edit these files (§7.13 Limit). These checks stop other users, repositories and mise or direnv redirection.

## 5. User config (account-user-config.ts)

- The top level must be a map, or the whole file is ignored with a warning. Keys: `default_name` and `folders`. An unknown key gets a warning and is ignored.
- `default_name` must be a valid account name; otherwise it gets a warning and is ignored. alz warns when `.claude-<name>` or `.codex-<name>` is in the $HOME listing.
- `folders` keys are absolute or start with `~/` (expanded from home). Values are a valid name (every target) or a map from target names to valid names (`default` allowed). Each bad entry gets a warning (key text through `quote`) and is ignored.
  - A key that names an account folder is ignored before any file system call.
  - A key that does not resolve matches nothing, silently.
  - Two keys with the same real path and different values are both ignored, with a warning.
- Match: the entries whose real path `isWithin` the real cwd (whole segments). The longest real path wins.

## 6. Allow state (account-allow-state.ts)

- The top level is a list, or null for an empty file; anything else ignores the file with a warning.
- Each entry is a map with exactly `root` (an absolute string), `account` (a non-empty map from target names to valid names, validated as parseAccount does) and `source` (a non-empty string). Anything else ignores that entry with a warning; account names never appear in it.
  - A root that names an account folder is ignored before any file system call.
  - A root that does not resolve matches nothing.
  - Two entries with the same real root are both ignored, with a warning, so that root is not allowed.
- `entryFor(root)`: `realpath(entry.root) === root`, exact. A nested repository needs its own entry.
- `sameMap(a, b)`: the same set of targets and the same names.

## 7. Per-target resolution (account-resolution.ts)

For each target T with variable V and marker M = `ALZ_SET_<V>`. The first step that names an account wins. Later steps are not evaluated.

1. `run.target === T`: named(run.account), source `run`.
2. `use[T]`: named, source `use`. Otherwise skipped (`no-use-choice`).
- E. V is a string (an empty one included) and M ≠ V (or M is absent): source `environment`, apply `keep`, with one warning "V was set outside alz", plus "; mise or direnv is active here" when envManagerActive. This takes the place of steps 3 to 5. An unset V is never foreign, and a stale marker is ignored.
3. A user config match that names T: named, source `user config`. Otherwise skipped (`no-user-config` | `user-config-ignored` | `no-folders-match` | `entry-names-no-target`).
4. Layer:
   - No root: skipped (`no-repository` | `cwd-in-account-folder`). allowed.yaml missing or ignored: skipped (`allowed-missing` | `allowed-ignored`).
   - No entry for the root: skipped (`not-allowed`), and no file of the stack is opened. The allow command is not printed.
   - With an entry E:
     - Manifest unreadable or invalid: fail closed (`manifest`) for each target in E.account. Other targets are skipped without failing.
     - Cache location: envManagerActive, XDG_CACHE_HOME absolute, and `real(value) ≠ real(~/.cache)` give a miss `cache-location` (owner, 2026-11-22). Otherwise call `readAccountDeclaration({ repoRoot: root, stack, cacheDir: xdgBase(XDG_CACHE_HOME, ~/.cache)/agent-layerz, home, env })`.
     - On a miss, check whether the recorded source is still in the stack. Skip each stack entry whose `sourcePathCandidates` name an account folder, or that has a `..` git segment. If any other entry has `recordedSource(e, root) === E.source`, fail closed with `cache-miss` and the recorded account per target in E.account: a recorded `default` or default_name unsets the variable; otherwise it is set to the recorded folder, with `folderMissing` when that folder is not listed. If none matches: `source-gone`, blocked.
     - `conflict`: blocked for each target in E.account.
     - Read without a declaration: `changed` with `now` absent, blocked for each target in E.account.
     - Read with a declaration D, `sameMap(D.account, E.account)` and `D.recordedSource === E.source`: named(D.account[T]), source `layer`. A target not in the map is skipped (`layers-name-no-target`).
     - Read with a declaration that differs: `changed`, blocked for every target in keys(E.account) and keys(D.account), with one warning that shows the old and new map and source (sources through `quote`). A ref bump leaves the source the same, so the account still applies.
     - Each declaration in `ignored` gets one warning: layer name and source through `quote`, plus its reasons. It never changes the result (AC 6).
   - The stack is read at most once, and only when some target reaches step 4.
5. Default: account `default`, apply `unset`. It never fails, even when ~/.claude or ~/.codex is missing.

- named(name): `default` or default_name gives the default account (apply unset, source kept). Otherwise the folder is `join(home, ".claude-"|".codex-" + name)`. If the listing has it, apply `set`. If not, `missing-folder` with apply set to that folder and `existing` = the listed names.
- Blocked: apply set to `~/.claude-alz-blocked` or `~/.codex-alz-blocked`. Nothing is created.

## 8. alz account which

- `alz account which [--use <target>=<name>]...`. `--use` is how the alz shell function passes its choice; nothing in the environment is read as one. An unknown target, a bad name, the same target twice or a positional argument exits 2 with the usage text.
- stdout (fixed phrases; the tests assert on them):
  ```
  repository: ~/work/c/api
  accounts.yaml: ~/.config/agent-layerz/accounts.yaml (read)
  allowed.yaml: ~/.local/state/agent-layerz/allowed.yaml (read; this repository is allowed)
  claude: client-c (source: layer) CLAUDE_CONFIG_DIR=~/.claude-client-c
    use step skipped (no alz use choice)
    user config step skipped (no folders entry for this directory)
  codex: default (source: default) CODEX_HOME unset
    ...
    layer step skipped (the layers name no codex account)
  ```
  - Foreign value: `claude: set outside alz (source: environment) CLAUDE_CONFIG_DIR="<displayPath>" kept`.
  - Default with a default_name: `default, called <name> here`.
  - Fail closed:
    ```
    claude: fails closed (source: layer)
      layer step skipped (<reason>); fails closed
      the hook applies CLAUDE_CONFIG_DIR=<folder> (<the missing folder | the recorded account <n> | the blocked folder>)
      fix: <command>
    ```
  - Missing folder: `<step> step names <n>, but <folder> does not exist; fails closed`, then `fix: alz account init <n> (account folders here: a, b)`.
- stderr: warnings through formatDiagnostic (`warning [account-<code>] ...`), each once.
- Exit codes: 0 when every target resolves, with or without warnings; 1 when any target fails closed; 2 for a usage error.
- Quoting:
  - Paths (root, files, folders, foreign value) go through displayPath.
  - Repository and layer text (sources, layer names, user-config keys, allowed.yaml sources) goes through `quote`.
  - Account names are validated, and reasons are fixed phrases.
  - Nothing is ever evaluated, so a directory named `$(touch pwned)` is only printed.

## 9. Spec changes in the same PR (LAYERING-SPEC.md)

§7.13:
- Paths: the walk rule from section 3 (unreadable or invalid manifest; never opening layer.yaml at $HOME, above it or in an account folder).
- Both files:
  - An empty or relative XDG_* value counts as unset.
  - Both files are read up to 1 MiB with the alias limit.
  - A path that names an account folder, or a platform with no uid, means the file is ignored.
  - The git work tree is found by lstat of `.git`.
  - Shape errors and duplicate roots or paths are ignored with a warning.
- No fetch on resolve: XDG_CACHE_HOME under mise or direnv is a cache miss (`cache-location`).
- Fails closed:
  - Folder existence comes from the $HOME listing (a directory or a link), and `alz-blocked` never counts as existing.
  - An unreadable manifest and a conflict apply the blocked folder.
  - A recorded `default` unsets the variable on a cache miss.
  - The fix per case, including `alz account allow --remove` when the declaration is gone.
- Values set outside alz: an unset variable is never foreign; an empty one is.
- `alz use` stays in the shell: the argument is `--use <target>=<name>`.

§10: the which row is built (TASK-50.10), with `--use` and the exit codes. §13 test 19: name `test/acceptance/31-account-which.test.ts`, and add the unreadable-manifest and cache-location cases. README: the which row. Release note: new command, no change to output or layer format.

## 10. Tests

- test/fs-trace.ts (new): test 30's interception (`trace`, `intercept`, `traced`, plus an async `tracedAsync`), moved out of test 30, which then imports it. If vi.mock hoisting blocks a shared module (`await import` inside the factory), copy the block instead.
- test/fixtures/31-account-which/: the layout of fixture 30 (personal, hub/layer as client with a per-target map, group), plus:
  - repo/sub/deeper/.gitkeep;
  - repo/.layer/instructions/ (a layer manifest on the way);
  - repo/nested/ with its own stack layer.yaml and .layer/.
  The hub becomes a git repo with tags v1 and v2 at test time, as in test 30.
- Every test uses a fake HOME and sets XDG_* explicitly, with no `__MISE_DIFF`, `DIRENV_DIFF` or session markers unless a case sets them. Tests create file and directory modes explicitly (0600/0700/0755), because a CI umask could make them group-writable. Tests write allowed.yaml and accounts.yaml themselves, and create account folders with mkdir in the fake home.
- test/acceptance/31-account-which.test.ts:
  1. Sources (AC1): default with no files (both file paths printed, "not found"); user config (string and per-target, `codex: default`); layer (a matching allowed.yaml); use (`--use`, beating user config and layer, and `--use codex=default`); environment (a foreign value kept with one warning; a matching marker is not foreign; `--use` replaces it); default_name maps to default and warns when its folder exists. A forged ALZ_USE_CLAUDE changes nothing. A bad `--use` exits 2. Resolver level: `run` beats `use`.
  2. Paths (AC4):
     - whole segments (client-a against client-ab), and the longest entry wins;
     - a symlinked cwd and a symlinked folders key;
     - a case variant (`it.skipIf` on a case-sensitive file system);
     - a nested manifest is its own root, and the outer entry does not apply there;
     - the cwd at repo/.layer/instructions finds repo;
     - a folders key that does not resolve matches nothing;
     - unreadable manifests on the way (invalid YAML, over 1 MiB, past the alias limit, a symlink, a directory, mode 000 skipped as root) stop the walk: an allowed root fails closed with the blocked folder, a root that is not allowed gives the default and never the outer repository's account.
  3. Files (AC2):
     - inside the repo (XDG_CONFIG_HOME pointing into it);
     - group-writable file, folder, and ~/.config (a parent below HOME);
     - not owned (resolver level with uid+1);
     - XDG_CONFIG_HOME and XDG_STATE_HOME away from their defaults with `__MISE_DIFF`, then with `DIRENV_DIFF`: ignored, the warning names the variable and says mise or direnv is active; the default location is still read under mise;
     - an allowed.yaml entry with a relative root, a bad name or a duplicate root is ignored;
     - a file in a git work tree gives a warning and is still read;
     - a marker string in `__MISE_DIFF` and `DIRENV_DIFF` appears in no output;
     - resolver level: a Proxy env records every `get`, and none is for `__MISE_DIFF`, `DIRENV_DIFF` or ALZ_USE_*; every get is on the list in section 2;
     - build output and the lock are byte-identical with and without accounts.yaml, allowed.yaml, CLAUDE_CONFIG_DIR and `__MISE_DIFF`.
  4. Allow (AC5): no entry gives the default, with no layer.lock or cache file opened (trace). A matching entry gives the named account. A changed name, an added target, a removed target or a changed source fails closed with the blocked folder and `fix: alz account allow`, with one warning showing the old and new values. A ref bump (build at v2) keeps the account. A source no longer in the stack is blocked. A declaration that is gone gives `alz account allow --remove`. A conflict is blocked.
  5. Fail closed (AC1):
     - a missing named folder from use, from the user config and from the layer (the hook applies the missing folder; the fix lists existing names);
     - a missing ~/.claude or ~/.codex never fails;
     - a cache miss (cache entry deleted) with the source still in the stack applies the recorded account, `fix: alz build`;
     - cache-location under mise;
     - exit code 1 in each of these cases, and no ~/.claude-alz-blocked is created.
  6. Reads only (AC3), the trace on `alz account which` run in-process:
     - no child_process call;
     - only read-only fs functions, every openSync read-only and O_NOFOLLOW;
     - the opened paths lie within {walk manifests, the two real files, the lock, the index entry, the tree2@ or git entry, the personal layer, .layer/layer.yaml};
     - readdirSync only on HOME and the layer directories being hashed;
     - no path inside a planted `~/.claude-client-a/` except the HOME listing;
     - `listing()` of the repository, the cache and the fake HOME is unchanged;
     - import graph from account-resolution.ts and commands/account.ts (type-only imports skipped): no git.ts, tree-cache.ts, sources.ts, pipeline.ts or child_process.
  7. Quoting: a cwd named `$(touch pwned)` with an ESC in a parent folder, and a hub layer name or source with ESC in the changed-entry warning: no control character in stdout or stderr, and no file `pwned` created.
- test/acceptance/24-untrusted-repo.test.ts (AC6): with an allowed root, a repo-layer declaration added after the build (inside, never hashed) and a group layer declaring in a cache entry and lock written by the test (test 30's lockEntry approach) each give exactly one warning line from `alz account which`. The client's allowed account still applies, and the resolver result lists it in `warnings` once whatever the number of targets.
- Run: `npx vitest run test/acceptance/31-account-which.test.ts test/acceptance/30-account-cache-read.test.ts test/acceptance/24-untrusted-repo.test.ts`, then `mise run test`, `mise run lint`, `mise run build` and `mise run alz:check`. Record the commands and results in the notes.

## 11. Threat-model checks that apply

1. Paths: no writes at all. Every path from repository text (stack sources) or the environment (XDG_*, CLAUDE_CONFIG_DIR, CODEX_HOME) is checked with namesAccountFolder before any file system call, and with touchesAccountFolder once it is real.
2. Subprocesses: none, proven by the trace and the import graph.
3. Locks: read only through the TASK-50.9 reader (one entry per source, validated sha, content hash).
4. Secrets:
   - `__MISE_DIFF` and `DIRENV_DIFF` are checked for presence only (Proxy test plus the output marker);
   - no account folder file is opened, only the $HOME listing;
   - no env value other than the account variables reaches output;
   - no account value goes into the build output (byte-identical test).
5. Security floor: an account named by a layer applies only with an exact allowed entry. A refused declaration never applies. Every doubt fails closed or skips step 4, never into a newly declared account.
6. Symlinks: a layer.yaml on the walk is never followed (O_NOFOLLOW) and stops the walk. The two config files may be links: they are read at their real path, and both the link's folder and the target's folder are mode-checked. Cached layers are not followed (reader).
7. Untrusted text to the terminal: displayPath and quote; nothing is evaluated.
8. Denial of service: 1 MiB per manifest or file, at most 100 aliases, the reader's 2000 entries and 16 MiB, a walk bounded by path depth, the stack read at most once.
9. TOCTOU: inode and dev equality on open; the directories are mode-checked; only the user (an accepted Limit) can swap them.

Residual risks, recorded in the spec's Limit:
- An agent running as the user can edit both files.
- HOME itself set by mise or direnv is not detected. It moves every account folder too, so a tool then starts logged out.
- The trust table in the threat-model skill still calls the environment trusted. TASK-50.11 AC3 updates it.

## 12. Order of work

1. core helpers (files, env, xdg, sanitize) and account-folders additions.
2. account-root.ts, account-files.ts, the parsers, environment.
3. account-resolution.ts and account-messages.ts.
4. commands/account.ts and main.ts.
5. fs-trace extraction, test 31 and its fixture, the test 24 cases.
6. Spec, README.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 owner: when a layer.yaml on the root walk cannot be read (permission, over 1 MiB, invalid YAML), the walk stops there, treats that folder as the root, warns, and fails closed if that root is allowed; otherwise the default account applies. It never skips past an unreadable manifest.

2026-11-22 coordinator: include the rule that a non-default XDG_CACHE_HOME while mise or direnv is active is a cache miss (fails closed in an allowed repository), consistent with the owner's XDG decision.

2026-11-22 coordinator, applying the owner's earlier decisions to the architect's questions: (1) the folders holding accounts.yaml and allowed.yaml must also be owned by the user and not group- or world-writable; (2) an unreadable layer.yaml in an allowed repository makes the hook apply the blocked folder (unknown state starts logged out, decision-16); (3) when the recorded account for a target is default and the cache is stale, the hook unsets the variable (the approved account is the default); (4) alz account which exits 1 when any target fails closed; (5) the release holds until every TASK-50 subtask is Done, so fix lines may name alz account init and alz account allow; (6) the root-walk ruling is recorded as decision-19; the XDG_CACHE_HOME rule follows the owner's XDG decision in TASK-50.2's notes.

2026-11-22 implementer: built the resolver (src/resolve/account-resolution.ts and the account-*.ts modules: root, files, user-config, allow-state, environment, messages, warning) and 'alz account which' (src/cli/commands/account.ts, main.ts routes 'account which [--use <target>=<name>]...'). Core helpers: RegularFile gains uid and mode, envManagerActive (presence only, via the property descriptor, never env[name]), core/xdg.ts, displayPath, listAccountNames/accountFolder/blockedFolder. Spec §7.13, §9.1-adjacent text, §10 which row, §13 tests 19/20/24 and README row updated; 'planned' removed only for the resolver and 'which'. Tests: test/fs-trace.ts (shared trace, moved out of test 30), test/account-workspace.ts (shared setup), test/acceptance/31-account-which.test.ts (+ fixture 31) and two cases in test 24. Checks: mise run test (30 files, 648 passed, 1 skipped), mise run lint, mise run alz:check all passed. Deviations: the fail-closed wording for which (hook applies, fix) is in commands/account.ts, not account-messages.ts, so alz run, the hook and doctor must extract it when they need it; extra modules account-warning.ts and test/account-workspace.ts; the coordinator's directory-owner rule is implemented (root accepted outside HOME) but untestable without chown, so only the group/other-writable directory cases are tested; XDG_CACHE_HOME empty/relative is treated as unset by the resolver while build still uses the old cacheRoot (TASK-53).

2026-11-22 implementer, review fixes: S1 mise/direnv XDG check now runs before the existence check; S2 the walk stops only inside an account folder or at/above HOME, a root that merely contains CLAUDE_CONFIG_DIR/CODEX_HOME stays a root and step 4 is a cache miss (account-folder); S3 O_NONBLOCK on every read open plus a FIFO test; S4 run/use names are validated (warn, skip step); N1 allowed roots must equal their real path; N2 the walk's real cwd is reused and the user config step is skipped in an account folder; N3 relative HOME ignores both files and fails named accounts closed; N4 envManagerActive uses Object.hasOwn only (tests use accessor properties and traps); N5/N1 limits in spec; N6 note added to TASK-50.6; N7 stripControl in main.ts; fail-closed wording moved to account-messages.ts; duplicate allowed roots give one warning. Not done as asked: a sync FIFO hang cannot be timed out in-process, so the test relies on lstat and O_NONBLOCK being asserted on every open.

2026-11-22 implementer, final review fixes: R1 a working directory inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names finds no root and opens nothing there; allowed.yaml entries whose real root is the cwd or a parent fail closed on the blocked folder (kind configured-folder); allowed entries in a configured folder are kept at parse. A: bad run/use names fail closed (kind invalid-name). Spec reconciled and the Limit lists the cases where an allowed repository gets the default account. B: note added to TASK-50.6. Test nits fixed.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-22T03:35:00Z

---

Owner question (affects criterion #4, the root walk; blocks the plan of this task only). The spec does not say what the walk does when a layer.yaml on the way cannot be read: over 1 MiB, past the alias limit, invalid YAML, or not a regular file. Options: (a) stop there and treat that directory as a root whose stack cannot be read: step 4 is skipped, and if that root is in allowed.yaml it fails closed like a cache miss (recommended); (b) skip it and walk on. Then a nested checkout with a broken manifest inside an allowed repository would get the outer repository's account.

---

author: @architect
created: 2026-11-22T07:25:00Z

---

decision-18 (owner, from TASK-50.9): layers inside the repository are read only to warn and never checked against the lock. Criterion #3 says 'checks each against the lock's content hash'; that now means each layer outside the repository (owner to reword if wanted). Step 4 calls readAccountDeclaration from TASK-50.9 (plan section 4): pass the real root, the parsed stack and cacheDir; map its miss codes to the 'layer step skipped (<reason>)' text; use recordedSource() for the 'recorded source still in the stack' check on a miss.

---

author: @architect
created: 2026-11-22T08:19:00Z

---

Owner questions on the plan. Each has a default in the plan, so implementation is not blocked; say if you want another option.
1. Directory owner (plan section 4). The spec checks the owner of accounts.yaml and allowed.yaml only. Options: (a) as written (plan default); (b) also require that the file's folder and each parent below $HOME are owned by you (root accepted outside $HOME), so another local user cannot swap the file. Recommend (b): cheap and stricter.
2. An unreadable or invalid layer.yaml at an allowed root (plan section 7). The plan applies the blocked folder, because it cannot confirm the recorded source is still in the stack. Alternative: the recorded account, as on a cache miss. Recommend blocked.
3. A recorded 'default' (or default_name) on a cache miss. The plan unsets the variable (the approved account is the default; run and use still fail closed). Alternative: the blocked folder. Recommend unset.
4. Exit code of alz account which. The plan uses 1 when any target fails closed, 0 otherwise, 2 for usage errors. Alternative: always 0, since it is a report. Recommend 1.
5. The fix lines name alz account init (TASK-50.5) and alz account allow [--remove] (TASK-50.11). This assumes all TASK-50 slices ship in one release. Confirm.
Also decided and noted in the plan: duplicate roots or paths are ignored with a warning; an empty account variable without a marker is foreign, an unset one never is; alz-blocked never counts as an existing folder; the walk never opens layer.yaml at $HOME, above it or in an account folder. Discovered: TASK-53 (build's cacheRoot with an empty or relative XDG_CACHE_HOME).

---
<!-- COMMENTS:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
resolveAccounts resolves the account per target for alz run, alz use and the hook: run, use, a value set outside alz (ALZ_SET_ markers; mise or direnv named only by presence), user config folders (longest whole-segment match), the layer declaration if allowed (exact real root, same map and source), then default. It finds the root by walking to the nearest layer.yaml with a stack (decision-19: an unreadable one stops the walk and fails closed when allowed), reads accounts.yaml and allowed.yaml bounded with O_NOFOLLOW|O_NONBLOCK after every spec check (outside the repo and account folders, owned by the user with owned folders, not group or world writable, XDG redirection under mise or direnv refused, relative HOME refused), and fails closed: recorded account on a cache miss with the same source, blocked folder on a changed declaration, a gone source, a conflict, an unreadable manifest or a cwd inside a configured folder, missing named folders and invalid explicit names. alz account which prints each target, its source, skipped steps and fail-closed state; exits 0, 1 (fails closed) or 2 (usage). Never writes, fetches or runs git (test 31 traces it). Spec §7.13, §9.1, §10, §13; README command row. 682 passed, 1 skipped; lint and alz:check clean. Reviewed in three rounds by reviewer and security-reviewer, both ready.
<!-- SECTION:FINAL_SUMMARY:END -->
