---
id: TASK-50.9
title: Read a stack's account declaration from the layer cache without git
status: Done
assignee:
  - '@claude'
created_date: '2026-11-22T03:33:00Z'
updated_date: '2026-11-22T08:01:00Z'
labels:
  - area:accounts
  - area:resolve
  - area:security
milestone: m-3
dependencies:
  - TASK-50.8
references:
  - docs/LAYERING-SPEC.md
  - src/resolve/cache-integrity.ts
parent_task_id: TASK-50
priority: high
type: feature
ordinal: 26390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Second slice of TASK-50.2. Account resolution runs at every prompt after a cd (shell hook) and before every alz run, so it must never fetch, clone or run git, and must write nothing (LAYERING-SPEC §7.13 'No fetch on resolve'). This slice gives resolution a reader that finds each layer of an allowed repository's stack in the layer cache, or in place for an unpinned path: layer, checks it against layer.lock's content hash with TASK-51's verifyCachedLayer, and applies the first slice's who-may-declare rule. alz build gains the pathref@ cache index for pinned path: layers.

Anything the reader cannot verify is a cache miss (TASK-50.2 implementer note from the TASK-51 review): no lock entry, a different sha, a missing or malformed field, an unreadable entry, or a layer.yaml that is not a regular file. TASK-51's lockedEntryFor returns nothing for an unpinned path: layer (no sha), so the reader needs its own entry selection there.

No CLI command yet; the next slice calls the reader. Cache-only change: generated output and layer.lock do not change.

Spec: LAYERING-SPEC §5.3, §6, §7.13 'No fetch on resolve', §9.1 (the index lives in the cache), §13 test 20 (index cases).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A pinned path: layer outside the repository is found through a cache index entry that the commands resolving the stack write next to each tree2@<tree> entry, keyed by the layer directory's real path and the commit SHA (for example pathref@<sha256 of the real path>-<sha>) and naming its tree; build and resolution compute the real path with the same function, resolution computes it from the manifest without git, uses the lock sha only when it is 40 or 64 lowercase hex characters, reads only the index entry for that path, and uses it only when lstat shows a regular file whose whole content is a tree id as long as the commit's; a path: layer without a ref outside the repository is read in place and checked against the lock's content hash; a layer inside the repository (LAYERING-SPEC §7.12), whatever its source, is read only to warn, with only its layer.yaml, in place, never checked against the lock, and skipped when it cannot be read; anything else, a layer directory that does not exist included, is a cache miss, and layer.lock does not change (tests: index hit, a malformed sha, another path's index entry, a malformed index entry, a symlinked index entry, a missing directory, an inside layer edited or unreadable)
- [x] #2 The reader never fetches, clones or runs git and writes nothing; it reads only the layers it can locate as in criterion #1, with build's validation (layer.yaml a regular file, parsed only after the content check), checks each against layer.lock's content hash (TASK-51), and counts a mismatch, a missing lock entry or field, a lock sha that does not belong to the entry it read, or a cache entry that is not a real directory as a cache miss (tests trace git calls and file reads)
- [x] #3 For a stack it can read, the reader yields the winning account declaration (the per-target map, the declaring layer, its revision, and its source in the form allowed.yaml records: a remote URL without its ref, or a local layer's real path) and each declaration it ignores because its layer may not declare, by the same rule the build applies (tests: the client layer's map wins, a ref bump leaves the recorded source unchanged, a repo-layer declaration is reported as ignored)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Basis: the owner answered (notes, 2026-11-22; decision-18): a layer inside the repository is read only to warn about its declaration and is never checked against the lock. Facts: the reader relies on no Claude or Codex behaviour and runs no git. The index writer reuses the git calls TASK-51 already makes (rev-parse, the ls-tree directory check, cat-file --batch-check; LAYERING-SPEC §6), so docs/target-facts.md §12 is not relied on and no new fact is needed. Generated output, layer.lock and the layer format do not change: no version bump. The cache gains pathref@ files; add one line to the TASK-50 release note.

## 1. Spec (architect, in this PR; decided by the owner, so not editorial)
§7.13 'No fetch on resolve':
- Replace the unpinned path: bullet with: "a path: layer without a ref outside the repository: the layer in place. Its content hash is checked (below), so an edit since the last alz build is a miss." and "a layer inside the repository (§7.12), whatever its source, is read only to warn when it declares an account, which it may not. Only its layer.yaml is read, in place, as a regular file of at most 1 MiB with the YAML alias limit. It is never checked against the lock, and a layer.yaml that cannot be read or parsed is skipped, so editing .layer/ never changes or blocks the account. A git: layer inside the repository is not read."
- "A lock sha is used only when it is 40 or 64 lowercase hexadecimal characters and the lock has exactly one entry for the source. Resolution checks each layer outside the repository against the lock's content hash ..."
- Index: "every command that resolves a pinned path: layer (build and the other commands in §5.3) writes, next to its tree2@ entry, an index entry ... whose whole content is the tree id (lowercase, no newline). It is written through a temporary file and a rename, rewritten when it differs, and never removed."
- Add: "A repository root, cache or layer directory that lies inside an account folder, or contains one ($HOME or above it, or the folder CLAUDE_CONFIG_DIR or CODEX_HOME names), is a cache miss, and nothing in it is opened (§9.1)."
- Status header: "Built so far (TASK-50.8, TASK-50.9): ... the cache reader and the cache index."
§9.1: change "the cache index that alz build writes" to "that the commands resolving the stack write". §13 test 20: tag the cache-index and cache-miss cases "built at the reader level (TASK-50.9, test 30)".

## 2. Modules
a. src/resolve/cache-paths.ts (new):
   - cacheRoot(home, env), moved from sources.ts;
   - gitEntryName(cloneArg, sha) = `${gitCacheKey}@${sha}`;
   - treeEntryName(tree);
   - pathIndexName(realDir, sha) = `pathref@${sha256(realDir)}-${sha}`;
   - layerRealPath(repoRoot, pathText) = realpathSync.native(resolve(repoRoot, pathText)), or undefined. Build and resolution both use this one function;
   - isObjectId(text): /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/. lock.ts uses it too.
b. src/resolve/sources.ts:
   - Extract locateSource(entry, repoRoot), which returns {kind:"path", dir, ref?} | {kind:"git", cloneArg, remote, ref} | {kind:"invalid", reason}. It only parses (splitRef, GIT_REF, gitTarget) and touches only file metadata. resolveSource uses it, so build and the reader key a source the same way.
   - Export recordedSource(entry, repoRoot), the allowed.yaml form: path: gives layerRealPath; github: gives githubCloneUrl; a git: remote gives the URL as written, without @ref; a git: local source gives its canonical clone path. A ref bump never changes it.
   - materializeAtRef calls writePathIndex after the tree2@ entry has been verified, or written, verified and renamed.
c. src/resolve/path-index.ts (new) holds both sides of the format:
   - writePathIndex(cacheDir, realDir, sha, tree): nothing to do when lstat shows a regular file whose content is already the tree. Otherwise write the tree (no newline, mode 0644) to cacheDir/.tmp-<random hex> with flag "wx", then renameSync it over the name. The rename replaces a file or a link and never follows the link. insideRoot(cacheDir, ·) guards both names. A directory at the name, or a failed write, gives the §5.3 cache error ("delete it and rebuild"), and the temp file is removed.
   - readPathIndex(cacheDir, realDir, sha) returns {tree} or {miss: "index-missing" | "index-invalid"}. It needs lstat to show a regular file of 40 or 64 bytes. It opens with O_RDONLY|O_NOFOLLOW, fstat must show a regular file, the content must pass isObjectId, and its length must equal the sha's.
d. src/core/files.ts gets readRegularFile(path, maxBytes), returning Buffer | "missing" | "invalid": lstat must show a file within the limit, the open uses O_NOFOLLOW, and fstat must show the same regular file. src/core/yaml.ts gets parseYamlLimited(text), with an explicit maxAliasCount of 100. TASK-50.10's root walk reuses both.
e. src/model/lock.ts: split parseLock(text) out of readLock, with the same validation and errors.
f. src/resolve/account-folders.ts (new): touchesAccountFolder(realPath, home, env). It is true when realPath is realpath(home) or above it; or below realHome/.claude, .codex, .claude.json, .claude-* or .codex-*; or equal to, below or above the real path of a non-empty CLAUDE_CONFIG_DIR or CODEX_HOME. It uses only path arithmetic and realpath. TASK-52 reuses it for build.
g. src/resolve/cached-stack.ts (new): readCachedStack(input) locates and checks each stack entry (section 3).
h. src/merge/declarers.ts: widen evaluateDeclarations and declarerProblems to {source, meta} (a type change only). In src/merge/account.ts add evaluateAccount(layers, repoRoot), which is pure and writes no diagnostics. mergeAccount becomes evaluateAccount plus reportDeclarations; build behaviour does not change.
i. src/resolve/account-declaration.ts (new): readAccountDeclaration (section 4).
The reader modules (g, i and what they import) never import src/resolve/git.ts or node:child_process.

## 3. Reading each stack entry (in stack order; the first miss is returned)
0. If touchesAccountFolder(realpath of repoRoot or cacheDir) is true, the result is the miss account-folder. Read the lock with readRegularFile(<root>/layer.lock, 8 MiB) and parseLock: no lock is lock-missing, and anything else wrong is lock-unreadable.
1. locateSource. An invalid source (unsupported scheme, bad ref, a git: path with ..) is source-invalid.
2. Test whether the layer is inside the repository with isInsideRepo(repoRoot, source) (§7.12).
   - An inside path: layer has its layer.yaml read in place: layerRealPath, the account-folder guard, readRegularFile with 1 MiB, parseYamlLimited, then parseLayerMeta. Any failure skips the layer without a miss or a warning.
   - An inside git: layer is not read.
   - Inside layers are never hashed, and the lock is not consulted for them.
   - A skipped layer keeps its position as a placeholder that declares nothing, so index 0 is still the first stack entry.
3. Outside layers need lock entries with the same source: none is lock-entry-missing, and more than one is lock-entry-duplicate.
4. Find the directory:
   - git: the sha must pass isObjectId, or the miss is lock-sha-invalid. The directory is cacheDir/gitEntryName(cloneArg, sha).
   - path: with a ref: realDir = layerRealPath, or the miss is layer-directory-missing. The sha must pass isObjectId, or the miss is lock-sha-invalid. readPathIndex gives the tree, and the directory is cacheDir/tree2@<tree>.
   - path: without a ref: the lock entry must have no sha (lock-sha-invalid otherwise). The directory is layerRealPath (layer-directory-missing otherwise). lstat must show dir/layer.yaml as a regular file before anything is hashed (layer-invalid otherwise), as build does.
   - A cache directory that lstat cannot find is cache-entry-missing, and one that is not a real directory (a link) is cache-entry-not-directory.
   - If touchesAccountFolder(realpath(dir)) is true, the miss is account-folder.
5. loadLayer(dir, source, scratch diagnostics, verify), where verify throws a private sentinel when verifyCachedLayer({contentHash}, entry) is "miss". That sentinel is the miss content-hash. loadLayer is unchanged, so the content is hashed before layer.yaml is judged or parsed. A FatalError, or any error in the diagnostics (a .env file), is layer-invalid. Any other file system error is unreadable.
6. When every entry has been read, validateStack runs over them with scratch diagnostics; an error is stack-invalid.
The reader never creates, writes, renames or deletes anything, never runs git and never lists the cache root. It builds cache paths only from hashes and validated ids.

## 4. The API TASK-50.10 calls: readAccountDeclaration(input)
It writes nothing, runs no git, emits no diagnostics, prints nothing and never throws because of repository or cache content.
Input:
- repoRoot: the real root from TASK-50.10's walk;
- stack: StackEntry[] from the manifest the caller already parsed;
- cacheDir: computed by the caller with cacheRoot(home, env);
- home;
- env: only CLAUDE_CONFIG_DIR and CODEX_HOME are read.
Output, one of:
- {status:"miss", reason, entry?: {index, source}}
- {status:"read", declared?, ignored, conflict?}, where:
  - declared is {account: AccountMap, layer: {index, name, source}, sha?, recordedSource}. sha is the lock's commit SHA, which §5.3 calls the revision; an unpinned path: layer has none;
  - ignored is a list of {layer: {index, name, source}, reasons: string[]} in stack order, including declarations by layers inside the repository;
  - conflict is {clients: [...], firstRefused: boolean} and comes with no winner, exactly as evaluateDeclarations decides.
reason is one of: account-folder, lock-missing, lock-unreadable, source-invalid, lock-entry-missing, lock-entry-duplicate, lock-sha-invalid, layer-directory-missing, index-missing, index-invalid, cache-entry-missing, cache-entry-not-directory, content-hash, layer-invalid, stack-invalid, unreadable. These codes are stable. TASK-50.10 maps them to 'layer step skipped (<reason>)' and the one 'run alz build' hint. Sources are raw repository text, so callers print them through quote or stripControl. recordedSource(entry, repoRoot) and cacheRoot are exported too, because when a read misses, TASK-50.10 must still check whether the recorded source is in the stack, without reading any layer.

## 5. The index writer: where and when
- Where: in the cache root, next to the tree2@ entries.
- When: in materializeAtRef, once the tree2@ entry is good. So every command that resolves a pinned path: layer (build, check, diff, doctor, ls, explain, lint, init) writes the entry or refreshes it. Nothing is written when resolution fails.
- Refresh: when the entry is already correct, nothing happens. It is rewritten when it is missing, differs, is malformed or is a link. A directory at the name is an error. Entries are never removed, and entries for other shas stay, like tree2@ entries. A wrong entry can only cause a miss, because resolution still compares the content with the lock.
- Atomic: the temp file sits in the same directory and is renamed. Two builds that race both write the same bytes, and the reader sees either no file or the whole file.
- The reader never writes an entry. A cleared cache or a sha that was never built is a miss until alz build runs.

## 6. Tests: test/acceptance/30-account-cache-read.test.ts
Fixtures in test/fixtures/30-account-cache-read/:
- personal/ (kind personal, public, account: personal-a, instructions/10-personal.md);
- hub/layer/ (kind client, private, account {claude: client-a, codex: client-a-api}, instructions/10-client.md);
- hub/docs/README.md;
- group/ (kind group, private, account: group-a);
- repo/layer.yaml, whose stack each test rewrites;
- repo/.layer/layer.yaml (kind repo) and repo/.layer/instructions/10-repo.md.
At run time, as in test 28: hub becomes a git repo tagged v1 and v2, with HOME=ws.home and XDG_CACHE_HOME=<ws>/cache. A helper writes git-<gitCacheKey("https://example.invalid/personal.git")>@<sha> from fixture files, plus a lock entry from hashLayerDir, so the reader reads a remote layer and rule cases are built without any network.

How the trace works (AC #2):
- vi.mock("node:fs") and vi.mock("node:child_process") use importOriginal. Every function they export is wrapped so that, while tracing is on, it records its name, its path argument (or the path behind an fd) and its open flags. realpathSync.native is kept.
- Positive control: the same trace must record git calls and writes during an alz build in the same file, which proves that the interception works.
- Around readAccountDeclaration the test asserts:
  - no child_process call at all;
  - no fs call outside the read-only allowlist: lstatSync, statSync, fstatSync, existsSync, realpathSync(.native), readlinkSync, openSync with read-only flags, readSync, closeSync, readFileSync and readdirSync;
  - content reads only of layer.lock, the expected pathref@ name, files under the located entries or in-place directories, and repo/.layer/layer.yaml;
  - no call of any kind on the decoys: another path's pathref@ entry with the same sha, another tree2@ entry and another git-…@ entry;
  - the cache root is never listed;
  - names, bytes and mtimes of the repo and the cache are the same before and after, and so are layer.lock's bytes.
- A static test confirms that the reader modules' import graph has no src/resolve/git.ts and no node:child_process.

Cases:
- AC #1:
  - an index hit;
  - build writes the index: the content equals `git rev-parse <sha>:layer` with no newline; a second build does not rewrite it (same inode and mtime); the next build rewrites a tampered index; a directory at the name fails the build with the cache error; generated output and layer.lock are byte-identical whether the index is kept or deleted and rebuilt;
  - a malformed sha (short or uppercase) is lock-unreadable;
  - a sha with no index entry (another commit) is index-missing;
  - another path's index entry (hub copied to hub2, only hub2 built, the stack names hub) is index-missing, and the decoy is not touched;
  - a malformed index entry (trailing newline, 39 characters, uppercase, not hex, or 64 characters for a 40-character sha) is index-invalid;
  - a symlinked index entry is index-invalid;
  - an unpinned path: layer outside the repository is read in place, and an edit to it is content-hash;
  - inside the repository: an edit to .layer/ still reads; a .layer/layer.yaml over 1 MiB, with invalid YAML or behind a link is skipped, not a miss.
- AC #2:
  - the trace assertions above;
  - an edited git entry or tree2@ entry is content-hash;
  - an invalid layer.yaml in the cache is content-hash, not layer-invalid, which shows it is parsed only after the content check;
  - a missing lock entry is lock-entry-missing;
  - no lock is lock-missing;
  - a lock entry without contentHash is lock-unreadable;
  - a duplicate entry is lock-entry-duplicate;
  - a pinned entry without a sha, or an unpinned one with a sha, is lock-sha-invalid;
  - a lock sha of v2 with v1's content hash, both cached, is content-hash (a sha that does not belong to the entry read);
  - a tree2@ or git entry that is a link to a good copy is cache-entry-not-directory;
  - a missing git entry is cache-entry-missing;
  - a symlinked layer.lock is lock-unreadable;
  - a layer directory at a fake $HOME with a layer.yaml and .claude-x/ is account-folder, and nothing under .claude-x is touched.
- AC #3:
  - personal declares a name and hub a map: hub's map wins, with layer hub, the sha, and recordedSource = realpath(hub/layer);
  - a ref bump from v1 to v2, then a build, keeps recordedSource and changes the sha;
  - a remote bumped from @v1 to @v2 keeps recordedSource as https://example.invalid/personal.git;
  - recordedSource of github:o/r@x is https://github.com/o/r.git;
  - a repo-layer declaration added after the build is ignored with "it is inside the repository", and the winner does not change;
  - a declaring group layer (built with the helper) is ignored with its reasons;
  - two client layers give no declared and list the clients under conflict;
  - evaluateAccount agrees with mergeAccount on the stacks used in test 29.
Tests 16, 24, 26, 28 and 29 pass unchanged.

## 7. Checks
mise run test; mise run lint; mise run build; mise run alz:check (this repo's generated config does not change).

## 8. Threat-model checks
1. Paths: cache names come only from sha256 hex and validated ids (the lock sha and the index content). insideRoot(cacheDir) guards the index write and its temp file. No repository text ever becomes part of a cache path.
2. Subprocesses: the reader starts none, which the trace and the import test both check. The writer adds none.
3. Lock: it is read as a bounded regular file and validated by parseLock. It needs exactly one entry per source, a valid sha and a matching content hash. The lock can only pick entries that alz wrote for that source and real path, and it can never make an unverified entry pass.
4. Secrets: miss reasons are codes, and results carry no file content. .env files are never read (hashLayerDir), and one in a layer makes the read fail.
5. Floor: the build's who-may-declare rule is reused unchanged, and inside layers never win.
7. Symlinks: the index entry and the lock are opened with lstat and O_NOFOLLOW. Cache entries must be real directories and layer.yaml a regular file, and listFiles never follows a link.
- §9.1: the account-folder guard, and the trace shows a fake account folder is never touched.
- Limits: the index is at most 64 B, the lock at most 8 MiB, and an inside layer.yaml at most 1 MiB with the alias limit.
- Accepted limit: an entry swapped between hashing and parsing needs write access to the user's own cache.
- Open (owner): an XDG_CACHE_HOME that a repo's mise or direnv sets. readAccountDeclaration takes cacheDir as input, so TASK-50.10 can apply the answer.

## 9. Out of scope
TASK-50.10: the root walk, accounts.yaml and allowed.yaml, the resolution order, alz account which and the miss texts. TASK-50.11: allow. TASK-52: the same account-folder guard in build.

## 10. Ambiguities and how this plan resolves them
- §7.13 says that once built, a second client layer's declaration is 'ignored'. TASK-50.8's rule gives no winner when two client layers declare or when the first layer is refused. This plan follows the code and reports conflict; the owner question is below.
- 'build writes' versus every resolving command: section 5, with the spec wording in section 1.
- 'Hexadecimal' now means lowercase, as in §5.3.
- 'The lock sha does not belong to the entry it read' means the content hash is compared with the one lock entry whose sha named the directory that was read.
- 'Revision' means the commit SHA, as §5.3 defines it.
- The real-path key: build passes the physical cwd and resolution passes the real root, through the same function. If the two ever differ, the result is a miss, never a wrong read.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 owner: layers inside the repository are read only to warn when one declares an account; resolution does not check them against the lock, so editing .layer/ never blocks the account.

2026-11-22 coordinator, following the owner's earlier decisions: (1) the spec follows the code: when two client layers declare, neither applies (no winner; build fails, resolution reports a conflict). Update §7.13 in this PR. (2) A non-default XDG_CACHE_HOME while mise or direnv is active counts as a miss, the same rule as accounts.yaml and allowed.yaml; built in TASK-50.10. (3) The account-folder guard in the reader is accepted; TASK-52 covers build.

2026-11-22 implementer: built per the plan. New: src/resolve/{cache-paths,source-location,path-index,account-folders,cached-stack,account-declaration,layer-load}.ts, src/core/env.ts; readRegularFile (core/files), parseYamlLimited (core/yaml), parseLock (model/lock), evaluateAccount (merge/account). Moved out of sources.ts and stack.ts so the reader never imports git.ts: source parsing and isInsideRepo (source-location.ts), loadLayer and validateStack (layer-load.ts); sources.ts and stack.ts re-export them, so tests 16, 24, 26, 28 and 29 pass unchanged. Env now lives in core/env.ts (git.ts re-exports the type). The index is written at the end of materializeAtRef (every resolving command). Evaluation gained firstRefused. Reader outputs sanitise the ignored reasons (they can carry a layer kind); sources stay raw for callers to quote. Spec: §5.3, §6, §7.13 (status, who may declare incl. two-client conflict, No fetch on resolve), §9.1, §13 tests 20 and 24 updated. Test 30 (63 cases) includes the fs and child_process trace with a positive control and an import-graph check; both were shown to fail when the reader was given a git call and a write. Open: the plan's 'one line in the TASK-50 release note' has no target, TASK-50 has no release note section yet.

2026-11-22 implementer, review round: B1 fixed (placeSource tells an unresolvable directory from an outside one; unresolvable gives layer-directory-missing, inside-means-unknown stays only in isInsideRepo/declarerProblems). S1: lexical, case-insensitive account-folder check (namesAccountFolder) on every manifest path before any fs call, canonical clone path of a local git: source checked too, a '..' in a git: URL is source-invalid. S2: chosen in the reader only (src/resolve/reader-load.ts), build untouched: every file is read once through readRegularFileWithId (O_NOFOLLOW, fstat match) and the same bytes are hashed and parsed; layer.yaml identity is compared with the lstat taken before hashing. N2: limits of 2000 entries and 16 MiB per layer (unreadable), 1 MiB for layer.yaml; in spec §7.13. N3: status 'conflict'. N4: inside-repo declarations are only ignored, never decide (reader strips them from the evaluation); conflict no longer carries firstRefused. N5: raw-text docs on the result types. N6: cache-in-repo. Review 1: lock-ref-mismatch. Review 3: index write failure other than a directory is warning [cache-index]. Review 4: the race branch verifies the entry another process made. Spec: §5.3, §6, §7.13 updated. AC #1 reworded to match decision-18. FOLLOW-UP (N1, no code): compare (dev, ino) of each ancestor instead of path strings in touchesAccountFolder and the repo/cache checks, because the /System/Volumes/Data spelling on macOS is unverified. Known limit: gitTarget probes a local git: source with existsSync (metadata only) before the canonical clone path is known, so a link inside the repository that points into an account folder is stat'ed, never opened.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-22T03:35:00Z

---

Owner question (affects this task's reader and TASK-50.10; blocks the plan of this task only). LAYERING-SPEC §7.13 'No fetch on resolve' checks every layer resolution reads against the lock's content hash. That includes the repository's own unpinned `path:./.layer`, so in an allowed repository any edit under `.layer/` since the last `alz build` is a cache miss: alz run refuses, and the hook keeps the recorded account until `alz build`. A layer inside the repository can never declare, so its content cannot change the resolved account. Options: (a) as written: hash every layer; (b) layers inside the repository are read only to warn about an ignored declaration (layer.yaml only, size-limited) and never count as a miss; (c) do not read layers inside the repository at all, which drops the 'ignored with one warning' case for the repo layer from §13 test 24. Recommendation: (b). Layers outside the repository are hashed either way.

---

author: @architect
created: 2026-11-22T07:25:00Z

---

decision-18 records the owner's answer to comment #1 (option b). The plan is written; the spec text for §7.13 and §9.1 in plan section 1 goes in this PR. TASK-52 (discovered) covers the same account-folder guard for build.

---
<!-- COMMENTS:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
readAccountDeclaration reads a stack's account declaration from the layer cache without git, fetches or writes: git layers from git-<hash>@<sha>, pinned path: layers through a pathref@ index that alz build writes atomically next to each verified tree2@ entry (a failed write is a warning), unpinned outside path: layers in place; each outside layer is read once with O_NOFOLLOW, hashed and checked against layer.lock (source, sha, and for pinned path: layers the ref), within 2000 entries and 16 MiB, then parsed. Layers inside the repository are only read to warn (decision-18). Anything unverifiable is a miss with a stable reason (including layer-directory-missing, account-folder, cache-in-repo, lock-ref-mismatch); two client layers give status conflict. Repository paths that name an account folder are refused before any file-system call. Spec §5.3, §6, §7.13, §9.1, §13. Test 30 (80 cases) traces fs and child_process calls to prove no git and no writes. 555 passed, 1 skipped; lint and alz:check clean. Reviewed twice by reviewer and security-reviewer, both ready. Follow-ups: TASK-52 (symlink-planted paths, NFC), N1 (dev/ino comparison) in notes.
<!-- SECTION:FINAL_SUMMARY:END -->
