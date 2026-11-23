---
id: TASK-55.4
title: 'R3: Source resolution and the cache in Rust'
status: Done
assignee:
  - '@architect'
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-23T12:12:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.3
parent_task_id: TASK-55
priority: medium
ordinal: 36390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R3: path:, git: and github: sources with the guarded clone, the cache layout and integrity checks, safe reads with openat. Builds TASK-52 and TASK-53 properly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Rust tests cover path:, git: and github: resolution with the guarded clone and its explicit child environment, the cache layout and integrity checks and the openat reads, and each case of tests 16, 24, 28 and 30 about them has a Rust test with the same verdict or a reachable CLI case (list in the notes)
- [x] #2 The cases of tests 16, 24, 28 and 30 that alz lint can reach (stack mode now resolves the layers, and R2's error [not-ported] is gone) pass against the Rust binary; the rest of those files pass by R5 (TASK-55.6)
- [x] #3 The spec Limits that openat closes are narrowed
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Scope

doc-5 R3 covers four things:
- `path:`, `git:` and `github:` sources, with the guarded clone and the explicit child environment (decision-24);
- the cache layout and its integrity checks (spec §5.3, §6);
- reads relative to a directory handle (openat), with trust checks by `fstat` on the handle that is read (decision-27);
- stack-mode `alz lint`, which now resolves the layers, so R2's `error [not-ported]` goes away.

By decision-30, R3 also fixes TASK-52, TASK-53, TASK-63, TASK-68, TASK-69, TASK-70 and TASK-71, in Rust only. R4 depends on them. The design starts from the spec (decision-22, doc-6, gr-rust). `src/resolve/` is read for behaviour only and never changed.

R3 relies on no Claude Code or Codex fact. The git facts it relies on are decision-24's evidence and target-facts §11:
- `clone` looks up its argument as a local path first;
- `protocol.file.allow=never` blocks that fallback, `GIT_ALLOW_PROTOCOL` overrides it, and `GIT_CONFIG_COUNT` does not;
- the local-clone CVEs.
Each new git behaviour R3 relies on (setsid leaves no controlling terminal, the `ls-tree -r -z` and `cat-file --batch-check` formats) gets a SMOKE run in the subtask that uses it. It is recorded in that subtask's notes and in the spec, not in target-facts, which holds agent facts only (the lead's note on TASK-63).

**Lock `rev`, confirmed** from `src/resolve/sources.ts`:
- `git:` and `github:` write the first 12 hex characters of the commit, in both branches of `resolveGit`;
- a pinned `path:` writes the manifest's `ref`;
- an unpinned `path:` writes nothing.

All three pass R2's `GitRef` rule, so the lock reader stays as it is. The resolver builds the short form with an infallible `ObjectId::short_rev() -> GitRef`, never by parsing text.

### Module layout (doc-6 §2)

alz-core, pure:
- `cache.rs` + `cache/{location,entry_name}.rs`:
  - `CacheLocation::from_env(home, xdg_cache_home)`: an empty or relative `XDG_CACHE_HOME` counts as unset (TASK-53). It is the one function that lint, build and R6's account resolution use.
  - `EntryName = Git { key, commit } | Tree(tree id) | PathIndex { dir_key, commit }`. It prints `git-<sha256 hex>@<sha>`, `tree2@<tree>` and `pathref@<sha256 hex>-<sha>`, and is never the old `tree@` form.
- `stack/origin.rs`: where an entry's layer comes from, found without git:
  - `Origin = WorkingTree(dir) | PinnedTree { dir, git_ref } | Clone { arg: CloneArg, transport: Remote | Local, git_ref }`.
  - `github:` gives `https://github.com/<o>/<r>.git`.
  - `git:` is a sans-IO classification: `NeedsLookup(candidates with git's suffixes /.git, "", .git/.git, .git, .bundle) | FileUrl | Remote | Invalid(DotDot)`. alz-io answers the lookups.
- `stack/resolved.rs` + `stack/validate.rs`: `Stack<Resolved>` (doc-6 §3.3).
  - `ResolvedLayer { entry, meta: LayerMeta, files: LayerFiles, pin: Option<Pin { rev: GitRef, sha: ObjectId }>, read: WorkingTree | Fetched | Cached }`.
  - §6 step 2: visibility order and duplicate names, which consumes the layers and returns the stack.
- `layer/scan.rs`: `LayerScan`, what the walk saw: entries, invalid names, `.env` names, digests and bytes. `scan_verdict(Purpose, &LayerScan)` encodes the §5.3 order of checks. `Purpose::Resolution` carries the 2000-entry and 16 MiB budgets for R6, and `Purpose::Build` has none.
- `lock/integrity.rs`: `locked_entry_for(&Lock, &Source, &ObjectId)` and `verify(&LayerFiles, &LockedLayer) -> Integrity { Matches, Differs }`.
- `git.rs` + `git/{environment,version,failure,tree,blob_id}.rs`:
  - `child_environment(GitOperation, &EnvVars) -> EnvVars` (TASK-63);
  - `GitVersion` and the floor;
  - `FetchFailureLine::from_stderr(&[u8]) -> Option<Stripped>`, steps 1 to 5 of §6 "A failed fetch" (TASK-68);
  - `TreeListing::parse` of `ls-tree -r -z`: each path must be UTF-8 (TASK-69) and a `SafeRelPath`; links and submodules are skipped; paths equal after NFC and lowercasing are refused;
  - `TreeEntryCheck`, which compares a scanned entry with the listing: NFC names, executable bit, no extra file, skipped names;
  - `BlobId::of`: `blob <len>\0` plus the bytes, hashed with SHA-1 (sha1, decision-36) or SHA-256 to match the commit's id.
- `account/folders.rs` + `path/walk.rs`:
  - `AccountFolders` from HOME, CLAUDE_CONFIG_DIR and CODEX_HOME; `touches()` compares per component in NFC (unicode-normalization, decision-37), lowercased.
  - `RealPathWalk`, a sans-IO state machine that resolves a path one component at a time. It asks `Lstat(prefix)` or `Readlink(prefix)`, and refuses a prefix that is `$HOME` or above, lies in an account folder or holds one, before it asks anything about it (TASK-52).
- `diagnostic.rs`:
  - codes `source`, `cache`, `cache-index` (a warning), `layer`, `env-file`, and `git` for the version floor;
  - message constructors made from `&'static str` and `Stripped` only;
  - `Code::NotPorted` is removed.

alz-io, effects:
- `env.rs`: `EnvSnapshot`, taken once in main, with typed queries, and the `EnvVars` given to git.
- `fs/dir.rs`: `Dir`, a directory handle that only the component walk produces. Each component is opened with `openat(O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC)`. A link is followed only by the walk, through `readlinkat`, with each prefix checked, and trust checks use `fstat` on the handle that is used. `ReadOnlyDir::open` stays for paths a person typed (the working directory, `--layer`). A `path:` directory from a manifest never goes through it (R2.7's note).
- `fs/scan.rs`: `Dir::scan(Purpose) -> LayerScan`, recursing with openat.
  - Names are `OsStr`, tested with `to_str`. Skipped names are never entered.
  - Links and other entries are listed but never read, and a `.env` name is never opened.
  - Files are opened with `O_NONBLOCK` and must show `S_ISREG` on `fstat`. Bytes are counted as they are read.
- `fs/cache_dir.rs`: `CacheDir`, the capability for the cache root.
  - It is opened or created through the walk, with `mkdirat` mode 0700 for each missing component, and refused inside an account folder (TASK-52) or, for an absolute XDG_CACHE_HOME, inside the repository (answer 6).
  - `entry(&EntryName) -> Option<Dir>`: a link or a non-directory at that name is the cache error.
  - `TempEntry` is `.tmp-<pid>-<n>`, made with an exclusive `mkdirat` 0700 and removed on drop. `publish` uses a `renameat` that does not replace. When the name exists, that entry is checked like a cached one.
  - The index is written and read through a temporary file and a rename. A failed write is a `cache-index` warning; a directory at the index name is the cache error.
- `git.rs` + `git/{runner,spawn,detach}.rs`: `GitRunner`, below.
- `resolve.rs` + `resolve/{working_tree,clone,pinned_tree}.rs`: the drivers. They carry out alz-core's decisions with `Dir`, `CacheDir` and `GitRunner`, and return `Stack<Resolved>` or diagnostics.

alz, the binary:
- `command/lint.rs` stack mode, in this order:
  1. read the manifest and the lock (R2);
  2. `alz_io::resolve::stack`;
  3. lint each layer's meta, then §6 step 2;
  4. print the summary `lint ok: <names>`.
- `main.rs` first passes argv to `alz_io::git::detach::run_if_requested()` (TASK-70), before clap.

### Capabilities: how alz-io runs git

`GitRunner::new(&EnvSnapshot)` runs `git version` once and refuses a git below 2.30 with error [git] naming the found and the minimum version (owner answer 4). Account resolution and the shell hook never get a `GitRunner`, so they cannot run git (decision-24).

A call is a typed `GitCall`, never strings from the caller. Its operands are `GitRef`, `ObjectId`, `CloneArg` and `SafeRelPath`, so an operand git could read as an option does not type-check:
- `Clone { guard, arg, into: &TempEntry }`: `clone --quiet -- <arg> <temp>`;
- `RevParseCommit`: `rev-parse --verify --quiet --end-of-options <rev>^{commit}`;
- `Checkout`: `-c advice.detachedHead=false checkout --quiet --end-of-options <sha>`;
- `ShowToplevel` and `ShowPrefix`;
- `LsTreeAt` and `LsTreeRecursive` (`-r -z`);
- `CatFileBatchCheck` and `CatFileBatch`.
`--` comes only before paths.

The environment: `env_clear()`, then exactly what `child_environment` returns:
- the snapshot without GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE, GIT_OBJECT_DIRECTORY, GIT_ALTERNATE_OBJECT_DIRECTORIES, GIT_COMMON_DIR, GIT_NAMESPACE and GIT_CEILING_DIRECTORIES, for every cache and clone operation (TASK-63). It also removes GIT_CONFIG_PARAMETERS, GIT_CONFIG_COUNT and every GIT_CONFIG_KEY_n and GIT_CONFIG_VALUE_n (owner answer 2).
- a remote clone: no GIT_ALLOW_PROTOCOL, and `-c protocol.file.allow=never`;
- every call: GIT_TERMINAL_PROMPT=0;
- every tree read: GIT_NO_REPLACE_OBJECTS=1 and GIT_NO_LAZY_FETCH=1.

Everything else passes unchanged, so ssh agents, proxies, GIT_SSH_COMMAND and credential configuration keep working. git is found through the child's PATH, which `test/git-proxy.ts` needs.

No terminal (TASK-70; owner answer 1):
1. `GitRunner` starts `current_exe()` with a fixed internal first argument.
2. That child is not a process-group leader. It calls `rustix::process::setsid()`, a safe function in rustix 1.1.6 that needs the `process` feature, which decision-27 already allows.
3. It then becomes git through `CommandExt::exec`, which is also safe.

git and ssh then run in a new session with no controlling terminal, so they cannot open /dev/tty. There is no unsafe code, and the user's `core.sshCommand` is untouched. ssh's graphical askpass stays allowed, since it does not use the terminal. std's `CommandExt::setsid` is still unstable in 1.100.0 (feature process_setsid, issue 105376, read in the toolchain's std source).

Standard input is null except for `cat-file --batch`. stdout and stderr are read on two threads, each with a cap: stdout per operation, stderr its first 64 KiB.

A deadline per call is a `GitRunner` parameter: the binary uses 10 minutes for a clone and 2 minutes for any other call (owner answer 5), and Rust tests pass short ones. When it passes, `rustix::process::kill_process_group` ends the whole session's group, ssh included. The exit status is checked, and stderr is never printed except as the TASK-68 line.

### Cache layout and integrity (§5.3, §6; entry names unchanged)

- **Root**: `CacheLocation` (TASK-53), opened as a `CacheDir`. It is refused when it lies in an account folder or holds one (TASK-52). An absolute XDG_CACHE_HOME inside the repository is refused with error [cache], a link in the root's own path is followed with each prefix checked, and missing folders are created with mode 0700 (owner answer 6).
- **`git:` and `github:`**, entry `git-<sha256(clone arg)>@<commit>`:
  1. When the lock pins this source and the entry exists, it must be a real directory. Its content hash is checked against the lock before `layer.yaml` is read.
  2. Otherwise alz clones into a `TempEntry`, guarded when the source is remote, then runs `rev-parse` of `<lock sha or ref>^{commit}` (an `ObjectId`) and checks the commit out.
  3. If an entry already exists, its content hash is compared with the clone's, and the clone is discarded. A difference is the cache error. If no entry exists, the clone is published.
  4. `rev` is the short sha.
- **Pinned `path:`**:
  1. `--show-toplevel` and `--show-prefix`, then `rev-parse ref^{commit}`.
  2. `ls-tree` must show the prefix as `040000 tree`.
  3. `ls-tree -r -z` is parsed and checked, and `cat-file --batch-check` finds every blob.
  4. An existing `tree2@` entry is verified against the listing, whatever the lock says. Otherwise the entry is written from `cat-file --batch` blobs, each id checked in Rust, with modes 0644 or 0755, and verified before it is published.
  5. Then the `pathref@` index entry is written, and the lock is checked as in §5.3.
- **Unpinned `path:`**: the walk opens a `Dir`, with the TASK-52 refusal. It is a working tree and is not checked against the lock.
- **Every layer**: `layer.yaml` is a regular file, read through `Dir` with `O_NOFOLLOW`.
- **Errors**:
  - a git step fails: `source`, `cannot fetch '<source>': <line>` (TASK-68);
  - after git succeeded, a name that is not UTF-8: the §5.3 `layer` error;
  - a read or rename fails: `cache`, `cannot store '<source>' in the layer cache at <dir>`.

### Purity

Each decision is a pure function or a sans-IO state machine in alz-core: origin, entry name, lock lookup, integrity verdict, scan verdict, tree checks, environment, failure line, account-folder walk and stack validation. alz-io only answers requests (lstat, readlink, open, read, run git) and builds the values the core consumes. The laws are therefore tested without a file system, and the drivers need only integration tests that show the requests are answered faithfully.

### Tests

alz-core unit tests and proptest laws (decision-35):
- `child_environment` never holds a removed name for a cache or clone operation, always holds GIT_TERMINAL_PROMPT=0, keeps every other variable byte for byte, and applying it twice gives the same result;
- `FetchFailureLine` holds no control or format character and no LF, is trimmed, and is `None` exactly when no line survives steps 1 to 4;
- `EntryName` is injective over (clone arg, commit), and printing then parsing gives the same value;
- `TreeListing::parse(emit(x)) == x`, and no two accepted paths are equal after NFC and lowercasing;
- `RealPathWalk`: for any answers to its requests, no request names a path at or below an account folder, or `$HOME` or above (TASK-52 AC #3);
- `scan_verdict`: the first failing check in §5.3 order decides, for any order of names;
- `BlobId` matches git's vectors: the empty blob e69de29bb2d1d6434b8b29ae775ad8c2e48c5391, and its SHA-256 form.

alz-io integration tests:
- They run real git in temporary repositories. alz-test-support gains a `GitFixture`: a fake HOME, GIT_CONFIG_NOSYSTEM=1 and GIT_CONFIG_GLOBAL in the scratch directory. They are skipped, with a message, below the floor.
- Cases:
  - decision-27's list: links planted in a layer, a cache entry or a path component; a FIFO, a device or a folder in place of a file, without blocking; a trailing slash; a file that grows past the cap; a name swapped for a link between two calls;
  - clone, rev-parse and checkout failures, with a fake ssh (GIT_SSH) that writes ESC, CR, U+202E and invalid UTF-8;
  - a fake ssh that opens /dev/tty, run under `script(1)` so the test process has a terminal: the open fails and nothing reaches the pty (TASK-70), plus `getsid` showing a new session;
  - a sleeping fake ssh, killed at a short deadline;
  - a pre-commit hook in a main checkout and in a linked worktree, run with the hook's GIT_INDEX_FILE and GIT_DIR: the user's index and repository stay byte-identical (TASK-63).

The binary: assert_cmd tests for stack mode in `crates/alz/tests/lint.rs`.

### Acceptance cases R3 must pass against ALZ_BIN

None of the existing cases in 16, 24, 28 and 30 runs only `alz lint` in a repository:
- they run build, check, doctor, init or `account which` (R5, R6);
- 28's "lint refuses a tampered cached layer" builds first;
- 24's "lint of a single layer reports it too" checks settings (R4).

So R3.11 adds lint twins, `alz lint` in place of `alz build` with the same fixture and verdict. A suite helper `writeLock` writes a version 1 lock with the content hash from `test/spec/content-hash.ts`, so the lock cases need no build. The twins:
- **16**: resolves `path:../hub/layer` at layer-v0.1.0; hub commits after the tag change nothing; a ref the hub lacks is `error [source]`.
- **24**: a lock sha that is not a commit id; a manifest ref that reads as an option; a source with credentials (TASK-71, drift).
- **28**: every case in "git cache keys", "cached content is checked against the lock", "pinned path: layers are checked against git", "pinned layer trees", "layer.yaml" and "an untrusted lock", plus "lint refuses a tampered cached layer" on a hand-written lock: about 30 cases.
- **30**: the index entry and the empty or relative XDG_CACHE_HOME (TASK-53). `account which` stays with R6.
- **New cases**, `referenceDrift` where the reference fails:
  - TASK-52: a layer at `$HOME`, or reached through a link into `~/.claude-x`, and a cache root in an account folder;
  - TASK-53: empty and relative values;
  - TASK-63: the hook;
  - TASK-68: in 26, a remote that fails, and a fake ssh writing control characters;
  - TASK-69: a name with the byte 0xFF, Linux only;
  - TASK-71.

Every case in those files passes by R5 (TASK-55.6).

### Spec changes in the same PR (R3.1)

- §6, a new paragraph "Git subprocesses": argument arrays, `--end-of-options`, the environment rule with the names it removes (TASK-63 AC #3), git 2.30 or later with error [git] below it (answer 4), no terminal through the self re-exec and setsid (TASK-70 AC #1, answer 1), the deadlines (answer 5), and the hook case as reference drift.
- §6: an empty or relative XDG_CACHE_HOME counts as unset for every command (TASK-53), not only for resolution (§7.13). An absolute one inside the repository is refused, a link in the root's own path is followed, and missing folders are created with mode 0700 (answer 6).
- §6 and §9.1: every command that resolves the stack refuses, before it opens anything there:
  - a `path:` layer whose real directory is `$HOME` or above, lies in an account folder, or holds one;
  - a cache root in an account folder.
  Each error names the layer (TASK-52, drift).
- AC #3: the spec has no openat Limit paragraph. The Node-era text is in §7.13 "No fetch on resolve" (a real path computed by realpath, a link planted in the repository that is lstat'ed) and in §9.1. R3.1 rewrites both to describe the component walk and the fstat-on-handle reads, and lists the reference's realpath behaviour as drift.
- No change to the layer format, the generated output or the bytes of `layer.lock`. The changes users can see (refusals, no ssh prompt, the hook environment, the deadline, the git floor) go into the Rust cutover's version note (R8). R3 needs no version bump of its own.

### Untrusted inputs

- `layer.yaml`: path text, URLs, refs;
- `layer.lock`: source, rev, sha, contentHash;
- layer trees: names, links, FIFOs, sizes;
- remote repositories: tree paths, blobs, link and submodule entries;
- git's stderr, which carries what the remote sent;
- cache entries, which may have been edited;
- the environment: HOME, XDG_CACHE_HOME, CLAUDE_CONFIG_DIR, CODEX_HOME, and the GIT_* a hook sets.

### Subtasks

R3.1 to R3.11 (TASK-55.4.1 to TASK-55.4.11), in order. Each description is that step's plan. The owner's answers (2026-11-23) are in the notes; decision-36 and decision-37 record answer 3. The criteria of TASK-52, 53, 63, 69 and 71 name the lint twins (answer 7).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
decision-30: R2 closes TASK-59 to TASK-62 (YAML and ref bugs, fixed in Rust only).

Architect (2026-11-23): criteria restated per the owner's answer 6 in TASK-55.3's notes: R3 is judged by Rust tests plus the acceptance cases its CLI can reach, and the full suite files must pass by R5. The old single criterion named tests 16, 24, 28 and 30, which mostly run alz build (R5).

From R2.7 (2026-11-23): the Rust lock reader validates rev as a GitRef and requires visibility; confirm against what the resolver writes as rev (a ref or a 12-character sha prefix).

Lead (2026-11-23): TASK-68 to TASK-71 are fixed inside R3, so R4 (TASK-55.5) depends on them instead of R3, as TASK-59 to TASK-62 were for R2.

Architect (2026-11-23): plan written and split into R3.1 to R3.11 (TASK-55.4.1 to TASK-55.4.11).

Where the carried bugs land:
| Task | Fixed in | Acceptance and close |
|---|---|---|
| TASK-52 | R3.5 (walk law, AC #3), R3.6 (AC #1, AC #2) | R3.11 |
| TASK-53 | R3.5 (one helper, AC #3), R3.6 | R3.11, lint twins of AC #1 and AC #2 |
| TASK-63 | R3.1 (spec, AC #3), R3.2 (environment, AC #2), R3.3 | R3.11, hook reproduction (AC #1) |
| TASK-68 | R3.4 (line and messages, AC #3), R3.8 (AC #4, wiring) | R3.11, test 26 (AC #1, AC #2) |
| TASK-69 | R3.7 (walk and check order, AC #2, AC #4), R3.9 (tree paths, AC #3) | R3.11 (AC #1, Linux) |
| TASK-70 | R3.1 (spec, AC #1), R3.3 (AC #2) | R3.11 |
| TASK-71 | R2.6 (the rule), R3.10 (lint end to end) | R3.11 |

Checked while planning:
- `rev`: `git:` and `github:` write the commit's 12-hex prefix, a pinned `path:` writes the manifest ref, and an unpinned `path:` writes none (src/resolve/sources.ts). All three are valid GitRefs, so R2's lock reader is right.
- std's `CommandExt::setsid` is unstable in 1.100.0 (`#[unstable(feature = "process_setsid", issue = "105376")]`, read in the toolchain's std source).
- `rustix::process::setsid()` is a safe `pub fn` in rustix 1.1.6 (`src/process/id.rs:234`, read in the registry source, not run).
- None of the existing cases in 16, 24, 28 and 30 runs only `alz lint` in a repository, so R3.11 adds lint twins.

Owner questions. Each has a recommended default; the task carries `needs:owner-approval` until they are answered. R3.1 waits for Q1, Q2, Q4, Q5 and Q6; R3.5 and R3.9 wait for Q3.

**Q1, TASK-70: how git and ssh are kept off the terminal without unsafe code.**
- a) Set GIT_SSH_COMMAND="ssh -o BatchMode=yes". This replaces the user's core.sshCommand or GIT_SSH_COMMAND.
- b) A re-exec trampoline. alz starts itself with a fixed internal argument. That child is not a process-group leader, so it can call `rustix::process::setsid()` (safe), and then it becomes git through `CommandExt::exec` (safe). git and ssh have no controlling terminal; agents and the user's ssh configuration keep working. The cost is one extra exec per git call and a hidden internal argv form.
- c) Wait for std's setsid. It is unstable, so not in 1.100.
- d) A crate such as process-wrap. It brings nix, which decision-27 excludes.

**Recommendation: b), for every git call.** It is recorded as an amendment to decision-24.

Sub-question: with no terminal, OpenSSH may run an SSH_ASKPASS program (a graphical window) when DISPLAY is set. Keep that, since it does not reach the terminal, or also set SSH_ASKPASS_REQUIRE=never (OpenSSH 8.4 and later)? Recommendation: keep it, and say so in the spec.

**Q2: GIT_CONFIG_PARAMETERS and GIT_CONFIG_COUNT/KEY_n/VALUE_n for cache and clone calls.** decision-24 leaves this open. A hook started by `git -c … commit` passes them on.
- a) Remove them.
- b) Keep them.
- c) Remove them for local cache reads only.

**Recommendation: a).** The user's config files still apply, so credential helpers and insteadOf keep working. target-facts §11 shows that GIT_CONFIG_COUNT does not override `-c`. Whether GIT_CONFIG_PARAMETERS does is UNVERIFIED, and removing it settles that.

**Q3: new crates.** Each needs a decision, and the intake check runs in the subtask.
- **sha1 =0.11.0**: RustCrypto, MIT OR Apache-2.0, released 2026-03-27, MSRV 1.85, `default-features = false`, in alz-core. It computes SHA-1 blob ids for tree2@ verification, as decision-24's consequence foresaw; SHA-256 repositories use sha2.
  - Alternatives: `git hash-object`, which contradicts decision-24 ("ids computed in Rust"), or a hand-written SHA-1.
- **unicode-normalization =0.1.26**: unicode-rs, (MIT OR Apache-2.0) AND Unicode-3.0, released 2026-10-16. deny.toml already allows Unicode-3.0. It provides NFC for the tree-entry comparison, the collision rule and account-folder names.
  - Alternative: icu_normalizer, which is a much larger tree.

**Recommendation: both.** rustix's `process` feature (setsid, kill_process_group) is already covered by decision-27.

**Q4: the git floor.** `rev-parse --end-of-options` needs git 2.30 (decision-24's evidence). 2.24 is the floor for `checkout` only, so the lead's note on TASK-63 should read 2.30.
- a) Refuse a git below 2.30 with `error [git]`, from one `git version` call per run, made only when a source needs git.
- b) Refuse below 2.45.1, the version that fixed the local-clone CVEs.
- c) No check.

**Recommendation: a).** The 2.44 and 2.45.1 caveats stay as the spec already states them.

**Q5: deadlines on git calls.** The reference has none.
- a) None.
- b) Fixed: 10 minutes for a clone and 2 minutes for any other call, then `cannot fetch '<source>': git did not finish within <n> seconds`, with the process group killed.
- c) b) plus an environment variable that overrides them.

**Recommendation: b).** The deadlines are a `GitRunner` parameter, so Rust tests use short ones, and the spec needs no new variable.

**Q6: edge cases of the cache location (TASK-53).**
- Empty or relative XDG_CACHE_HOME counts as unset; TASK-53 settles that.
- HOME unset, empty or relative: use the same rule as §7.13 "Paths", in one shared function. Recommended.
- An absolute XDG_CACHE_HOME inside the repository:
  - a) refuse it for every command with `error [cache]`, as resolution already does (`cache-in-repo`);
  - b) allow it for build.
  - Recommendation: a).
- A link in the root's own path: follow it, since the person configured that path, with each prefix checked against the account folders. A link at an entry is never followed. Recommended.
- A missing root is created with mode 0700 for each missing component. The reference uses 0777 minus the umask. Recommended, and noted as a visible change.
- The `cache-location` rule for a cache moved by mise or direnv stays with resolution (R6). Build trusts it, as today.

**Q7: criteria that name `alz build` (R5).** These are TASK-52 AC #1, TASK-53 AC #1 and AC #2, TASK-69 AC #1 and TASK-71 AC #1. TASK-52 AC #3 and TASK-63 AC #1 name account resolution and `alz check` instead.
- a) Check them in R3 with `alz lint` twins, which resolve the same way, on the shared walk. Their build twins pass from R5. This is what R2 did for TASK-61.
- b) Keep those tasks open until R5, and drop R4's dependency on them.

**Recommendation: a).** The owner or the lead rewords the criteria, since agents do not change them.

Owner answers (2026-11-23): (1) TASK-70: alz re-runs itself with an internal argument; that child calls rustix's safe setsid() and then runs git, so neither git nor ssh has a controlling terminal; the user's ssh command is untouched and ssh's graphical askpass stays allowed. (2) Cache and clone git calls drop GIT_CONFIG_PARAMETERS, GIT_CONFIG_COUNT and its GIT_CONFIG_KEY_n/VALUE_n pairs. (3) sha1 =0.11.0 and unicode-normalization =0.1.26 approved, one decision each. (4) Refuse git below 2.30 with error [git] naming the found and minimum version. (5) Deadlines: 10 minutes for a clone, 2 minutes for other git calls, a GitRunner parameter; on expiry the process group is killed and the fetch fails. (6) Cache root: empty or relative XDG_CACHE_HOME is unset; an absolute one inside the repository is refused; a link in the cache root's own path is followed; a missing root is created with mode 0700. (7) TASK-52, 53, 63, 69 and 71 may close in R3 with lint twins of their build/check cases; the lead rewords their criteria; the build forms are checked again in R5.

Architect (2026-11-23): the owner's answers are folded into the plan and into subtasks R3.1, R3.2, R3.3, R3.5, R3.6, R3.9 and R3.11. Answer 3 is recorded as decision-36 (sha1 =0.11.0) and decision-37 (unicode-normalization =0.1.26), both accepted. Answer 7: the criteria of TASK-52, 53, 63, 69 and 71 now name the lint twins, and each task's last criterion says that the build or check form is checked again in R5 (TASK-55.6), and the account-resolution forms in R6 (TASK-55.7). R3.1's spec text is written, uncommitted, on the branch.

Architect (2026-11-23): the owner's answers are folded into the plan and into subtasks R3.1, R3.2, R3.3, R3.5, R3.6, R3.9 and R3.11. Answer 3 is recorded as decision-36 (sha1 =0.11.0) and decision-37 (unicode-normalization =0.1.26), both accepted. Answer 7: the criteria of TASK-52, 53, 63, 69 and 71 now name the lint twins, and each task's last criterion says that the build or check form is checked again in R5 (TASK-55.6), and the account-resolution forms in R6 (TASK-55.7). R3.1's spec text is written, uncommitted, on the branch.

Owner answers to the R3 security review (2026-11-23): (M1) a remote git: source may use only https and ssh: refuse '<helper>::' URLs at parse time; run remote clones with protocol.allow=never plus protocol.https.allow and protocol.ssh.allow, and GIT_PROTOCOL_FROM_USER=0; the user's own GIT_ALLOW_PROTOCOL can only narrow it. (L3) local git: sources are cloned with --no-local. (L6) a file over 16 MiB in a layer is an error in the build too. Lead decisions on the rest: the credentials check takes the user part before the first '@' of the first '/'-segment for the scp-like form (so git@host:repo@v1 is accepted and me:TOKEN@host/o/r and user:tok@host:repo@v1 are refused) and keeps the whole-text check for '://' URLs; never signal a reaped pid, and confirm the group before trusting it; resolve git to an absolute path once (no relative PATH entries); refuse a cache root not owned by the effective uid or writable by group or others; probe_file without following links, regular files only; iterative remove_tree; account folders reached through links are refused (lstat/readlink the named entries of HOME); a pinned layer's git top is checked against the account folders; git is started only when a source needs it; the environment is captured once in main.

Implementer (R3 review fixes, git/credentials/supervision; uncommitted):
- M1: source.rs refuses '<helper>::' (SourceError::TransportHelper, also L4). Remote clones: -c protocol.allow=never/https=always/ssh=always, GIT_PROTOCOL_FROM_USER=0 and, beyond the owner's recipe, GIT_ALLOW_PROTOCOL=https:ssh (narrowed user list if set). Reason: a user's protocol.ext.allow=always beats protocol.allow=never; only GIT_ALLOW_PROTOCOL overrides every protocol.*.allow (real-git test fails with config alone, passes with the env var).
- M2: scp-like user part = before first '@' of first '/'-segment of the text without its @ref (ref only when no ':' and no '/'); '://' keeps the whole-text check. Known false positive: git:host:repo.git@feature/x (ref with '/'), documented in spec 5.2. Test 26 SOURCE restored.
- L3 --no-local for local clones. Nit: GIT_LITERAL_PATHSPECS=1 for TreeRead.
- L1/#3: spawn.rs rewritten: copy writes READY byte on stdout after setsid; group trusted only after it; end = group (if announced) then copy, then wait 1s for the byte and kill group again; after the copy's status no signal again; pipes read via chunk channels for a 0.5s grace then the result returns (partial streams kept).
- L2: git resolved once to an absolute path (alz-io/src/git/executable.rs), no empty/relative PATH entries; passed to the copy; version check on it.
- #2: Fetcher::on_demand; git started only for a clone (pinned path: still always needs git, pinned_tree.rs is the other implementer's). stack_lint test runs without git on PATH.
- #5: GitRunnerError::diagnostic, with_launcher returns GitRunnerError; Spawn -> the executable message.
- #7: deadline 10 s, pids in one file by rename, zombies count as dead, running_as_root() skip for 3 chmod tests. #8: EnvSnapshot and WorkingDirectory captured once in main.
- Acceptance: 24 gets scp-like and helper cases (helper cases referenceDrift TASK-55.4). Spec: 5.2 Transports/Credentials, 6 environment/Which git/abandoned calls, 7.12.
Checks: mise run rust:ci exit 0; mise run lint ok.

AC #1 mapping (2026-11-23). Read-only mapping at 4bc5132; no code changed. Abbreviations: PT=alz/tests/pinned_tree.rs, GS=alz/tests/git_sources.rs, SL=alz/tests/stack_lint.rs, GR=alz/tests/git_runner.rs, LINT=alz/tests/lint.rs, CD=alz-io/tests/cache_dir.rs, DH=alz-io/tests/directory_handles.rs, LS=alz-io/tests/layer_scan.rs, SRC=alz-core/src/stack/source/tests.rs, STK=alz-core/src/stack/tests.rs, LF=alz-core/src/lock/lock_file/tests.rs, EN=alz-core/src/cache/entry_name.rs (unit), INT=alz-core/src/lock/integrity.rs (unit). Crate prefix is crates/. "build form" = the same case through alz build: the resolution part is the lint twin, the output/lock/no-write part is R5 (task 55.6). Account which, init, doctor are R6 (55.7) unless stated.
Guarded clone, explicit child env, version floor, deadlines, no transport helper: GR::git_gets_a_cleared_environment_plus_the_child_environment_of_its_operation, a_remote_clone_runs_no_transport_helper_even_when_the_users_configuration_allows_it, a_remote_clone_never_widens_the_users_allowed_protocols_to_a_transport_helper, a_local_clone_copies_the_objects_and_never_hard_links_them_to_the_source, a_git_below_2_30_is_refused_..., a_call_past_its_deadline_is_killed_with_its_whole_process_group_ssh_included. path:/git:/github: resolution: PT (pinned path:), GS (git:), SL::a_git_layer_from_a_file_url..., a_git_layer_at_a_local_path..., a_github_layer_is_cloned_through_the_users_git_configuration, an_unpinned_path_layer_is_read_in_place...; openat reads: DH, LS, alz-io/src/fs unit tests.
== 16-hub.test.ts (7 cases)
hub lint: each repo resolves path:../hub/layer at layer-v0.1.0 -> alz/SL::a_pinned_path_layer_is_read_from_the_cache_and_checked_against_the_lock (tag ref, tree2@ entry, summary) + PT::a_ref_and_the_commit_sha_name_the_same_entry
hub lint: commits and edits after the tag do not change the verdict -> alz/SL::a_pinned_path_layer_is_read_from_the_cache_and_checked_against_the_lock (working-tree edit after the tag ignored) + PT::a_docs_commit_moves_the_commit_but_not_the_tree_and_the_entry_is_still_verified. Thin: no case where a layer.yaml committed (invalid) after the tag is ignored; same mechanism, tree read at the commit.
hub lint: a ref the hub does not have is an error [source] -> alz/PT::a_ref_the_layer_did_not_exist_at_and_a_directory_outside_a_repository_cannot_be_resolved + PT::the_pinned_layer_... line 682 "cannot resolve ref pinned for 'path:" + alz-core/src/stack/resolution_failure.rs unit (line 374). Thin: no alz-binary test of this message; acceptance with ALZ_BIN covers it.
hub build: each repo resolves at tag and grants the hub dir -> R5 (resolution covered by the lint twin; additionalDirectories, AGENTS.md, lock rev are output)
hub build: commits/edits after the tag do not change the build -> R5 (resolution covered by the lint twin)
hub build: a ref the hub does not have fails -> R5 (resolution covered by the lint twin)
doctor warns about CLAUDE.md at the client root -> R6 (doctor)
== 24-untrusted-repo.test.ts (10 in-scope case groups, 23 later-phase cases)
refuses a lock that lists a path outside the repo, deletes nothing [build] -> R5 (no-delete); the parse is alz-core/LF::a_file_entry_that_is_not_a_safe_path_with_a_hash_is_refused
refuses a lock whose layer sha is not a commit id [build] -> R5; parse is alz-core/LF::a_sha_that_is_not_an_object_id_is_refused_before_any_value_is_used + alz-core/src/lock/object_id.rs unit ("../../../etc/passwd")
never passes git:--upload-pack... / github:...@--upload-pack... to git [build] -> R5; lint twin below
rejects a manifest ref that git would read as an option [build] -> R5; lint twin below
lint: refuses a lock whose layer sha is not a commit id -> alz-core/LF::a_sha_that_is_not_an_object_id_is_refused_before_any_value_is_used (+ LINT::an_invalid_lock_with_a_valid_manifest_is_the_error_lock, which uses lockVersion 2, not a bad sha). Thin: no alz lint binary test with a bad sha.
lint: never passes %s to git as an option (git: and github: rows) -> alz-core/SRC::other_malformed_sources_are_refused_with_their_own_reason (git:-oProxy=x -> UrlStartsWithDash; github:...@--upload-pack=touch -> InvalidRef) + alz-core/stack/git_ref.rs unit. Thin: no binary test with the exact "git:--upload-pack=touch PWNED@v1" and PWNED unseen; SL::an_invalid_manifest_stops_before_any_layer_is_resolved only uses a list ref.
lint: rejects a manifest ref that git would read as an option -> alz-core/STK::a_ref_that_breaks_the_rule_is_refused_naming_the_entry_and_not_the_value ("--upload-pack=touch", "-x") + STK::a_ref_pins_a_path_source_and_is_checked_but_dropped_on_other_sources
credentials in a git: source, lint x6 sources (referenceDrift TASK-71) -> alz/SL::a_source_with_credentials_is_refused_before_git_or_the_cache_and_the_token_is_never_printed (the same six sources, git spy, no cache made) + SRC::credentials_in_a_git_url_are_refused, a_password_in_a_scp_like_address_is_refused_whatever_follows_it, STK::a_git_source_with_credentials_is_the_error_source_without_a_location_or_the_token. Build x6 -> R5.
does not refuse a user name alone for credentials (4 rows) -> alz/SL::a_user_name_without_a_password_is_not_refused_as_credentials (same four plus one) + SRC::a_user_name_alone_is_accepted_in_ssh_and_scp_like_urls; "cannot fetch '<source>'" text: GS::a_clone_that_fails_is_cannot_fetch_with_the_first_line_of_git_s_standard_error
transport helper (3 rows, referenceDrift TASK-55.4), lint -> alz/SL::a_transport_helper_url_is_a_manifest_error_and_git_never_runs (ext and https:: rows; token not printed) + SRC::a_transport_helper_url_is_refused_before_anything_else (all three incl. git:::x). Thin: the empty-helper row has no binary test, and the stack tree is not compared before and after. Build x3 -> R5.
account declaration by a layer that may not (lines 241-478, 5 cases), variable/passthrough/policy/names/MCP/one-report cases (479-803), refused declaration at resolution (805-868), hostile directory name from build and account which (869) -> R4/R5/R6 (merge/checks/build/account which), no Rust test needed for AC #1
== 28-cache-integrity.test.ts (33 guarantees, each twice: build and lint; the lint twin has 30)
git cache keys: never share a key between two URLs -> alz-core/EN::a_github_clone_is_named_after_the_hash_of_its_url_and_the_commit (SHA-256 vector) + the_same_clone_at_another_commit_is_another_entry; github URL: alz-core/src/stack/origin.rs unit
git cache keys: two repos with equal flattened paths in separate entries (build, lint) -> none: no test resolves two different git: sources and compares their entries; only the key hash is unit-tested. Add a GS test with two sources whose paths flatten the same.
git cache keys: a lock for one URL never reads another URL's entry (build, lint) -> alz-core/INT::the_lock_pins_a_source_by_the_sha_of_its_own_entry (lock lookup by source) + EN key per URL; none at the binary level (a lock for B holding A's sha).
cached content: untouched entry builds/resolves without error -> GS::a_layer_the_lock_pins_is_read_from_the_cache_without_cloning_again (build/check/diff are R5)
edited cached git layer names layer and entry -> GS::an_entry_that_differs_from_the_lock_stops_with_the_cache_error_before_any_git_call + SL::a_cached_git_layer_is_checked_against_the_lock_and_read_without_git_when_it_matches
file added to a cached git layer -> GS::a_file_added_to_a_cached_entry_is_a_difference_too
builds again once the edited entry is deleted -> GS::a_pin_the_cache_lacks_is_cloned_at_the_pinned_commit_though_the_ref_moved (missing entry, pin: clone again); no test of the exact delete-then-rebuild sequence
compares an existing entry with a fresh clone when the lock has no entry -> GS::an_existing_entry_is_compared_with_a_fresh_clone_when_the_lock_has_no_entry
accepts an untouched entry when the lock has no entry -> same test (the first half) and GS::a_git_layer_is_cloned_once_into_its_entry_with_the_twelve_character_rev
tampered layer.yaml gives the cache error and is never quoted -> none: every tamper test edits a.md; no test edits layer.yaml (invalid YAML marker) in a cached entry, and none checks it is not parsed before the check.
cache entry that is a link -> GS::an_entry_that_is_a_link_is_refused_with_and_without_a_lock + PT::an_entry_that_is_a_link_is_the_cache_error_and_is_never_followed + CD::an_entry_that_is_a_link_or_not_a_folder_is_never_followed
cached pinned path: layer edited / untouched entry rebuilds -> PT::an_entry_that_was_tampered_with_is_refused_on_every_read, PT::the_lock_is_checked_for_an_entry_read_from_the_cache_when_it_pins_the_commit
unpinned path: layer is a working tree, not checked -> alz/SL::an_unpinned_path_layer_is_read_in_place_and_never_checked_against_the_lock
pinned path: tampered entry after the commit moved but the tree did not; untouched after a docs commit -> PT::a_docs_commit_moves_the_commit_but_not_the_tree_and_the_entry_is_still_verified
pinned path: refuses an entry with an extra file / missing file / link / changed executable bit -> PT::an_entry_that_was_tampered_with_is_refused_on_every_read (5 rows: changed file, extra, missing, mode, link)
pinned path: never uses another layer's entry for a submodule at the layer path -> PT::a_link_or_a_submodule_at_the_layer_path_is_a_ref_that_cannot_be_resolved
pinned path: writes the tree's own bytes whatever attributes the working repo has -> PT::a_pinned_layer_is_written_to_its_tree_entry_from_the_committed_blobs (a .gitattributes with eol=crlf in the tree, working tree edited) + PT::the_blob_id_alz_computes_is_the_one_git_hash_object_computes. Thin: no test sets the repository's own .git/info/attributes or core.autocrlf.
older tree@ entries are ignored and the layer is written again as tree2@ -> none: no Rust test plants a tree@<id> folder.
skips links and submodule entries in the tree and expects them absent (then a link added to the entry is a change) -> PT::a_link_or_a_submodule_inside_the_tree_is_skipped_and_not_written + the link row of the tamper test
ignores .DS_Store and node_modules not in the tree -> PT::skipped_names_in_an_entry_are_ignored_unless_the_tree_has_them
names that differ only by case or Unicode form, quoted without ESC -> PT::two_names_equal_after_nfc_and_lowercasing_are_refused_before_a_write (no test of ESC removal in this message was checked)
file name not in NFC is built and checked again -> PT::a_name_that_is_not_in_nfc_is_written_and_verified_in_nfc
tree whose blobs the repository lacks, cache not read -> PT::a_repository_that_lacks_the_blobs_is_an_error_and_writes_nothing
replace refs ignored when an entry is written -> PT::a_replace_ref_does_not_swap_what_is_written
tampered entry with no layer.yaml gives the cache error -> none: no test deletes layer.yaml from a cached entry (the missing-file row removes docs/b.md).
layer.yaml must be a regular file, never a link -> LINT::a_layer_yaml_that_is_a_link_is_refused_and_never_read + LS::a_link_is_counted_and_never_followed_or_hashed + DH::a_link_inside_the_layer_is_never_followed_by_a_read
init: lint refuses a tampered cached layer -> SL::a_cached_git_layer_is_checked_against_the_lock_and_read_without_git_when_it_matches; init sets no identity from a tampered layer / sets it from an untouched one -> R6
untrusted lock: sha not a commit id before any cache path is built -> alz-core/LF::a_sha_that_is_not_an_object_id_is_refused_before_any_value_is_used + object_id.rs unit; the entry name is typed (EntryName), so no path is built from text
untrusted lock: refuses a lock with a numeric sha / non-string source / null contentHash / non-string name / bad name (5 rows) -> alz-core/LF::a_document_of_the_wrong_shape_is_refused_naming_the_field_and_no_value (bad name, bad source), a_content_hash_that_is_not_sha256_and_64_lowercase_hex_is_refused, a_sha_that_is_not_an_object_id...; numeric sha and non-string name are not row-for-row checked; binary [lock] verdict: LINT::an_invalid_lock_with_a_valid_manifest_is_the_error_lock
== 30-account-cache-read.test.ts (57 cases: 4 covered, 6 build-form R5, 47 account which R6)
lint twin: writes the index next to tree2@ with the tree id, not group/other writable, no tmp left -> PT::the_index_is_written_once_rewritten_when_it_differs_and_never_removed + CD::the_index_is_written_through_an_exclusive_temporary_file_and_a_rename (mode 0600, no tmp). Thin: "does not rewrite" (same inode and mtime on the second run) is not asserted anywhere.
lint twin: rewritten by the next lint when tampered file / link (2 rows) -> PT::the_index_is_written_once_rewritten_when_it_differs_and_never_removed + CD::a_folder_at_the_index_name_is_the_cache_error_and_a_link_there_is_replaced_not_followed + CD::only_a_regular_file_of_one_lowercase_object_id_is_an_index
lint twin: cache error when a directory sits at the index name -> PT::a_folder_at_the_index_name_is_the_cache_error (+ CD::...a_folder_at_the_index_name...)
lint twin: warning [cache-index] when the index entry cannot be written, no failure -> PT::an_index_that_cannot_be_written_is_a_warning_and_the_layer_still_resolves + CD::a_cache_that_cannot_be_written_is_the_cache_index_warning
build forms (index written by build; every command that resolves writes it; rewritten by next build x2 rows; build cache error at the index name; output and lock unchanged whether index kept or rebuilt; build warns when the index cannot be written) -> R5 (the library part is the lint twins above)
all other cases ("(through alz account which)": the pathref@ index read, misses on lock/entry/sha/hash/limits/links, fails closed on account-folder and ".." sources, declaration winners, layers inside the repo read only to warn, the git stand-in proof) -> R6. Note for R6: the read-only cache read with the lock check and miss reasons has no Rust code yet (only the write-side index, read_index and the openat primitives in alz-io); only read_index (CD) and the walk limits (LS) exist.
== counts
16: 3 covered (lint twins), 0 gap, 4 later phase (3 build forms R5, doctor R6); 2 thin spots
24: 6 covered (3 lint twins, credentials/lint, no-refuse, transport/lint), 0 gap, 4 build-only cases R5 plus the build halves of credentials and transport, 23 later (R4/R5/R6); 4 thin spots
28: 33 guarantees: 24 covered, 5 gap (flattened-paths keys, older tree@ format, tampered layer.yaml cache error, entry with no layer.yaml, repository-attributes variant as thin), 2 init cases R6, 2 partial (lock for another URL at binary level, delete-then-rebuild)
30: 4 covered (lint twins), 0 gap, 6 R5, 47 R6

AC #1 mapping correction (2026-11-23): the 28 count line is 33 guarantees = 27 covered (4 of them with a thin spot: repo-level attributes, a lock for another URL at binary level, delete-then-rebuild, ESC in the case-clash message), 4 gap (two repos with equal flattened paths; older tree@ entry; tampered layer.yaml in a cached entry; cached entry with no layer.yaml), 2 R6 (init). In the 16 hub note, the 'cannot resolve ref pinned for path:' line is at PT line 682, in the test on a pinned layer whose ref cannot be resolved (name not recorded here).

AC #1 gaps closed (tests only, crates/alz/tests/stack_lint.rs, lint twin of 28-cache-integrity): two_git_repositories_whose_flattened_paths_are_equal_are_kept_in_separate_entries; an_entry_of_the_older_tree_format_is_ignored_and_the_layer_is_written_again (also asserts the stale tree@ entry is left untouched); a_cached_layer_yaml_that_was_tampered_with_is_the_cache_error_and_is_never_quoted; a_cached_entry_without_its_layer_yaml_is_the_cache_error. All four passed at first run; mise run rust:ci exit 0. Not committed.

Lead (2026-11-23): AC #1 checked — the mapping notes give a Rust test for every resolution case of tests 16, 24, 28 and 30, the four gaps now covered by stack_lint.rs (equal flattened paths, older tree@ entries, tampered layer.yaml never quoted, entry without layer.yaml); build, merge, account-which and init cases belong to R4–R6. AC #2: every lint twin passes against target/release/alz (R3.11 and after the review fixes). AC #3: the spec Limits narrowed in R3.1 (§7.13, §9.1). R3 is set Done when PR #8 merges.

2026-11-23: PR #8 CI flaked once on ubuntu (git_runner a_git_at_2_30_is_accepted_and_asked_for_its_version_once at 4124a4e); green before and after (ba79e69). Tracked as a discovered task; the assertion now prints the refusal.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Source resolution and the layer cache are in Rust: a cleaned git environment and a GitRunner with typed calls, the 2.30 floor, deadlines and no terminal; https and ssh only for remote sources, --no-local for local ones; one cache location with owner-checked roots and temp entries; a no-follow component walk that never looks inside an account folder; the layer walk with its budgets and check order; git:/github: sources cloned once and pinned by the lock; pinned path: layers written from blobs and verified; and alz lint for a whole stack. Merged in PR #8 (rebase merge).
<!-- SECTION:FINAL_SUMMARY:END -->
