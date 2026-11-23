---
id: TASK-55.4
title: 'R3: Source resolution and the cache in Rust'
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-23T11:48:00Z'
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
- [ ] #1 Rust tests cover path:, git: and github: resolution with the guarded clone and its explicit child environment, the cache layout and integrity checks and the openat reads, and each case of tests 16, 24, 28 and 30 about them has a Rust test with the same verdict or a reachable CLI case (list in the notes)
- [ ] #2 The cases of tests 16, 24, 28 and 30 that alz lint can reach (stack mode now resolves the layers, and R2's error [not-ported] is gone) pass against the Rust binary; the rest of those files pass by R5 (TASK-55.6)
- [ ] #3 The spec Limits that openat closes are narrowed
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
<!-- SECTION:NOTES:END -->
