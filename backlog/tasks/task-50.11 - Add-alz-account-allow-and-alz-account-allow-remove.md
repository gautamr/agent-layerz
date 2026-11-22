---
id: TASK-50.11
title: Add alz account allow and alz account allow --remove
status: Done
assignee:
  - '@claude'
created_date: '2026-11-22T03:34:00Z'
updated_date: '2026-11-22T12:49:00Z'
labels:
  - area:accounts
  - area:cli
  - area:security
milestone: m-3
dependencies:
  - TASK-50.10
references:
  - docs/LAYERING-SPEC.md
  - .layer/instructions/50-repo.md
  - .layer/skills/threat-model/SKILL.md
parent_task_id: TASK-50
priority: high
type: feature
ordinal: 28390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Fourth and last slice of TASK-50.2. The only command that writes outside the repository and the cache: alz account allow, which a person runs to accept the account a repository's layers name (owner approval in TASK-50.2's notes; decision-16), and alz account allow --remove. It also adds the allow hint to alz build and alz check, and updates the repository invariant and the threat model, because the invariant 'outside the repo, only the cache is written' stops being true with this slice.

Spec: LAYERING-SPEC §7.13 'Allow state', 'Both files', 'Trust and alz account allow', 'Agent-session markers', 'Limit', §9.1 'Planned writes outside the repo and the cache', §10 'alz account allow', §13 test 20. Facts: docs/target-facts.md §12 item 19 (markers), item 21 (deny rules are not a guard).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 alz account allow shows the account per target, the declaring layer, its source and its revision, and asks the person to type the account name to confirm; it refuses and changes nothing when stdin is not a TTY or inside an agent session (any of the six markers in LAYERING-SPEC §7.13 'Agent-session markers' set, even to an empty value), and it has no --yes; it holds an exclusive lock file next to allowed.yaml while reading and writing, keeps entries sorted by root and replaces the entry for the same root; alz account allow --remove removes this root's entry under the same lock file, with the same terminal and agent-session checks; on a cache miss allow changes nothing and says only to run alz build (a test for each refusal and each marker, for --remove and for two concurrent runs)
- [x] #2 alz account allow records the repository's real root, the whole per-target map and the declaring layer's source (a remote URL without its ref, or a local layer's real path) in allowed.yaml, and only then does an account named by a layer take effect; alz build and alz check print the allow command only on a terminal with no agent-session marker set, and the hook and alz run never print it (tests: a fresh clone starts on the default account; after alz account allow it uses the named one; a changed map or source skips it again; a ref bump keeps it)
- [x] #3 The repository invariant 'outside the repo, only the cache is written' in .layer/instructions/50-repo.md names allowed.yaml as the one other file alz writes, the threat-model skill's trust table gains rows for accounts.yaml and allowed.yaml, and mise run alz:build then mise run alz:check pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Plan (@architect, 2026-11-22). Settled against the code of TASK-50.8 to TASK-50.10 (src/resolve/account-*.ts, src/cli/commands/account.ts, src/cli/main.ts, src/cli/io.ts, src/cli/bin.ts, build.ts, check.ts). Writer policy: decision-20 (proposed). Owner questions: comment on this task; the defaults below are the recommendations.

### 1. Person-at-terminal checks
- src/core/env.ts: AGENT_SESSION_MARKERS = CLAUDECODE, CLAUDE_CODE_CHILD_SESSION, AI_AGENT, CODEX_THREAD_ID, CODEX_SESSION_ID, CODEX_CI (target-facts §12 item 19, change C7) and agentSessionMarkers(env): the names that are set, by Object.hasOwn only (an empty value counts). Values are never read, printed or logged (same pattern as envManagerActive).
- src/cli/terminal.ts (new): interface Terminal { ask(prompt): Promise<string | undefined> } (prompt to stderr, one line from stdin, undefined at end of input or Ctrl-C); processTerminal(): a Terminal when tty.isatty(0), else undefined; readline is created lazily inside ask, so commands that never ask never touch stdin. personAtTerminal(io) = io.terminal !== undefined && no marker set.
- src/cli/io.ts: Io gains optional terminal?: Terminal. Absent means "standard input is not a terminal" (default deny). Only bin.ts sets it (terminal: processTerminal()). No other src file reads process.stdin, node:readline or node:tty.

### 2. One stack read for resolution, allow and the hint
- src/resolve/account-resolution.ts: extract readRepositoryDeclaration(root, stack, home, env) from resolveLayerStep (cacheLocation + readAccountDeclaration, miss "cache-location" included) and export it; resolveLayerStep calls it, behaviour unchanged. allow and the hint call the same function, so allow accepts exactly what resolution applies (§7.13 'Trust').
- src/resolve/account-declaration.ts: declared also carries ref (the stack entry's ref, raw text) for display.

### 3. allowed.yaml reader additions (no behaviour change for resolution)
- account-allow-state.ts: parseAllowState also returns shape ("list" or "not-list"), ignored (positions the reader warned about) and unresolved (valid entries whose root does not resolve, kept by the writer). New pure serializeAllowState(entries): header "# written by alz account allow; deleting the file withdraws every approval", entries sorted by root (compareCodePoints), keys root, account (target order), source, via stringifyYaml; an empty list is "[]".
- account-files.ts: export the checks the writer needs for a directory whose file does not exist yet (account-folder relation, inside the current repository, weakDirectory chain below $HOME), so reader and writer share one implementation. readAccountFile itself is unchanged.

### 4. The writer: src/resolve/account-allow-writer.ts (new; the only code that writes outside the repo and the cache)
updateAllowed({ change: set entry | remove root, repoRoot (real), home, env, uid, waitMs = 10000 }) returns written | unchanged | refused(reason), plus reader warnings. Steps:
1. Before any file system call: $HOME absolute, uid known, located path (locateAccountFile) not naming an account folder as written, XDG_STATE_HOME not moved away from ~/.local/state while __MISE_DIFF or DIRENV_DIFF exists. Any failure refuses, as the reader ignores.
2. Directory: from the nearest existing ancestor's real path, refuse when the directory would lie inside the repository or an account folder (nothing is created there). remove with no file: unchanged, nothing created. set: mkdirSync(dir, { recursive, mode 0o700 }); an existing directory keeps its mode (it must pass the reader's checks). Then realpath the directory and check again (account folder, inside repo, owner and group/others-writable chain below $HOME).
3. The file: absent, target = real dir + allowed.yaml. A link is kept: target = its real path, and both folders are checked (as the reader does). A dangling link, or anything that is not a regular file, refuses.
4. Lock: <target>.lock next to the file that is replaced, opened O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW, 0600, holding the pid. EEXIST polls every 50 ms up to waitMs, then refuses naming the lock file and saying to delete it if no alz account allow is running. A stale lock is never removed automatically; a lock that is a link is never followed.
5. Under the lock: readAccountFile("allowed.yaml", { home, env, uid, root: repoRoot }) again. ignored refuses with the reader's reason; a real path different from step 3 refuses. read: a top level that is not a list, or any entry the reader ignores (bad shape, relative root, invalid name, root not its real path, root in an account folder or at or above $HOME, two entries for one root), refuses, naming positions only, and says to fix or delete the file (decision-20; owner question 2).
6. New list: the valid and unresolved entries minus any whose real root equals repoRoot, plus the new entry (set) or nothing (remove). No difference (an equal entry, or nothing to remove): unchanged, no write.
7. serializeAllowState; refuse above 1 MiB (the reader would ignore the whole file); parse it back with parseYamlLimited and require deep equality, else refuse.
8. Temp file .<name>.<pid>-<8 random hex>.tmp in the target's directory, O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW, fchmod 0600 (umask-proof), write, fsync, close, renameSync onto the target. Any failure unlinks the temp file.
9. finally: unlink the lock only when lstat of the path still shows the inode of our fd.
10. Read back with the reader; the entry for repoRoot must equal the new one (or be gone), else exit 1 "written but does not read back".

### 5. alz account allow (src/cli/commands/account-allow.ts, new; main.ts dispatch)
main.ts: account allow [--remove], parseArgs strict, no positionals: --yes, -y or any other option is a usage error (exit 2). USAGE line added.
Order, each failure exit 1 with nothing changed:
1. No terminal, or any marker set: refuse before any file system call. Print every reason that applies, naming markers by name only. When CLAUDECODE is set: "Claude Code also sets it in an IDE's integrated terminal; run alz account allow in a terminal window outside the IDE".
2. HOME absolute; findRepositoryRoot(io.cwd, ...) (the same walk as resolution). No root refuses with the repositoryLine reason. A manifest that cannot be read refuses for allow ("repair <root>/layer.yaml").
3. readRepositoryDeclaration: miss -> "the layer cache cannot be verified (<missPhrase>); run alz build"; conflict -> remove one declaration, then alz build; no declaration -> "the layers name no account here" (adds alz account allow --remove when this root has an entry); a name alz-blocked -> reserved. Ignored declarations (layers inside the repo) are printed as warnings, as in which.
4. Pre-check, before the prompt: the reader on allowed.yaml (ignored refuses) and updateAllowed in check-only mode (its steps 1 to 3 without mkdir or any write), so a location or directory problem refuses before the person types. An equal entry for this root: "already allowed", exit 0, no prompt, no write.
5. Show on stdout: repository (displayPath); per target the account and whether ~/.claude-<name> or ~/.codex-<name> exists (listAccountNames, names only, §9.1), and the targets the layers leave out; the declaring layer (quote), its stack position, source as written (quote), ref (quote) when set, revision = the lock's commit SHA or "none (unpinned path: layer)", the source that will be recorded; the current entry with old values when it differs; one line: Claude Code and Codex started here will use these accounts' logins and MCP connections.
6. Confirm: one prompt per distinct account name in target order (owner question 3), via io.terminal.ask. The line without surrounding spaces and tabs must equal the name; y, yes, empty, anything else, end of input or Ctrl-C: "not confirmed; nothing changed". The answer is never printed back.
7. Read the declaration again; any difference from what was shown (map, recorded source, revision, layer) refuses: "changed while you were confirming; run it again".
8. updateAllowed set. Print the entry; note each named folder that does not exist yet, with the which fix line (alz account init <name>). Exit 0.

### 6. --remove
Steps 1 and 2 (an unreadable manifest is fine: the root is known); the stack is not read, so it works on a cache miss and after the declaration is gone. A missing file or no entry for this root: "nothing to remove", exit 0, nothing created. Shows the entry, asks for the recorded name or names like allow (owner question 1), updateAllowed remove, prints that the repository goes back to the default account unless accounts.yaml names one.

### 7. Build and check hints (src/resolve/account-allow-hint.ts, new, read-only)
allowHint({ repoRoot, stack, home, env, uid, declares }) returns note lines; build.ts and check.ts print them on stderr as "note [account-allow] ..." only when personAtTerminal(io), so no hint without a terminal or with a marker, and neither stdout nor any generated file changes.
- build: after a successful write. check: only when its own comparison passed (no change, collision or stale lock), whatever the identity result, because otherwise allow would hit a cache miss.
- No hint when the real root is at or above $HOME, or in or under an account folder or a configured folder (resolution never applies there).
- The layers declare and the root has no entry: the map, the layer name and "it applies only after you run: alz account allow". An entry that differs (map, target added or removed, recorded source): "was ... from <old>, now ... from <new>; run: alz account allow". The layers no longer declare and the root has an entry: "run: alz account allow --remove". allowed.yaml ignored: the reader's reason and "fix it, then run alz account allow". A cache miss: the miss phrase only.
- The hook and alz run (TASK-50.6, TASK-50.3) never call it.

### 8. Agent layer (run mise run alz:build, then mise run alz:check)
- .layer/instructions/50-repo.md invariant: "Every write and delete goes through insideRoot. Outside the repo, alz writes only the layer cache (~/.cache/agent-layerz) and ${XDG_STATE_HOME:-~/.local/state}/agent-layerz/allowed.yaml with its lock and temporary file, which only alz account allow writes, through src/resolve/account-allow-writer.ts. Account folders are read for names only."
- .layer/skills/threat-model/SKILL.md trust table, two rows:
  - accounts.yaml (${XDG_CONFIG_HOME:-~/.config}/agent-layerz/): trusted only when the §7.13 'Both files' checks pass; never written by alz. The owner writes it by hand, but any process running as the user (an agent) can edit it, so it is ignored inside the repo, in an account folder, when not owned by the user, writable by group or others, or located by an XDG_CONFIG_HOME moved while mise or direnv is active; names and paths in it are validated.
  - allowed.yaml (${XDG_STATE_HOME:-~/.local/state}/agent-layerz/): trusted only when the same checks pass, and an entry counts only while it equals the current declaration. Only alz account allow writes it, after a person types the account name on a terminal outside an agent session; the writer refuses every state the reader ignores. An agent that can write outside the repo can still edit it: the sandbox is the guard (§7.13 'Limit', target-facts §12 item 21).
- Also (owner question 4): checklist item 1 names allowed.yaml as the exception to insideRoot; new item 8 "Person-only commands refuse without a TTY on stdin or with any agent-session marker, have no --yes, and read the answer only through Io.terminal"; the environment row adds that markers prove presence only and that XDG_* moved while mise or direnv is active is not trusted.

### 9. Docs
README Commands table: alz account allow [--remove] (docs rule: README lists only commands the CLI has). main.ts USAGE.

### Spec changes in the same PR
(Editorial tag fixes TASK-50.2 -> TASK-50.11 in §9.1, §10 and §13 test 20 were made at plan time.)
- §7.13 header and the 'Allow state', 'Both files' and 'Trust' bullets: allow, --remove and the hints built (TASK-50.11).
- 'Both files' / §9.1: lock file next to the file that is replaced, 10 s wait, never broken automatically; temp file name; missing directories created 0700, an existing one keeps its mode; file 0600; link kept and its real path written; refuses every state the reader ignores, a dangling link, any ignored entry, a result over 1 MiB, a failed read-back; [] after the last remove.
- 'Trust': what is shown (revision = lock SHA, ref, recorded source, folder existence), one prompt per distinct name, answer matching, re-read after confirming, already allowed exits 0, no declaration / conflict / alz-blocked refuse, --remove confirmation, exit codes 0, 1, 2.
- 'Trust' hint bullet: check prints only after its comparison passes; stderr note; ignored file, removed declaration and cache-miss forms; no hint at or above $HOME or in account folders. Resolve the contradiction: "The shell hook and alz run never print it" becomes "never print it for a repository that is not allowed; alz run names alz account allow only as the fix when it fails closed on a changed declaration or source (below)" (§10 alz run already says so).
- §9.1 'Planned writes' -> 'Writes outside the repo and the cache', allow built. §10 allow row: drop Planned, add the above. §13 test 20: built in test/acceptance/32-account-allow.test.ts; test 24: the allow cases built.

### Files touched
src/core/env.ts, src/cli/terminal.ts (new), src/cli/io.ts, src/cli/bin.ts, src/cli/main.ts, src/cli/commands/account-allow.ts (new), src/cli/commands/build.ts, src/cli/commands/check.ts, src/resolve/account-resolution.ts, src/resolve/account-declaration.ts, src/resolve/account-allow-state.ts, src/resolve/account-files.ts, src/resolve/account-allow-writer.ts (new), src/resolve/account-allow-hint.ts (new); test/helpers.ts (optional terminal on layers()), test/account-workspace.ts (fixture name parameter), test/acceptance/32-account-allow.test.ts (new), test/fixtures/32-account-allow/ (new), test/acceptance/24-untrusted-repo.test.ts, test/acceptance/31-account-which.test.ts; .layer/instructions/50-repo.md, .layer/skills/threat-model/SKILL.md and the files alz:build regenerates (AGENTS.md, CLAUDE.md, .claude/skills/threat-model/SKILL.md, layer.lock); docs/LAYERING-SPEC.md; README.md.

### Untrusted inputs
- Repository content: layer.yaml on the root walk, the stack, layer.lock and cached layers, read only through readRepositoryDeclaration (lock sha check, content hash before parse, no git). From it: account names (validated), layer name (validated, printed with quote), source and ref (raw, printed with quote, stored as YAML data only), revision (40 or 64 hex), recorded source (a real path or URL text, data only). No repository text becomes a path alz writes to: the written path comes from HOME, XDG_STATE_HOME and fixed names.
- allowed.yaml (an agent may have edited it): the reader's checks under the lock, refusal on anything it ignores, read-back after the write.
- Environment: XDG_STATE_HOME and XDG_CACHE_HOME (mise or direnv may set them from repository content: refused or a miss), HOME, CLAUDE_CONFIG_DIR and CODEX_HOME (account folder relation only), markers and __MISE_DIFF / DIRENV_DIFF (presence only).
- File system state around the state dir: links, FIFOs, modes, owners, a planted lock or temp name (lstat, realpath, O_NOFOLLOW, O_EXCL, O_NONBLOCK reads in the reader).
- The typed line: compared only, never stored or printed. The working directory path: displayPath.

### Tests
How the tests drive a TTY: an injectable terminal (Io.terminal), not a pty helper. Node has no pty in core; node-pty is a native addon (a new dependency, intake, a compile per platform); script(1) differs between BSD and util-linux; Python's pty module is not guaranteed on a runner; and a test skipped when the tool is missing passes vacuously there. It is not vacuous because:
1. Default deny: no terminal object means refusal; every existing test and helper passes none, so the default path everywhere is the refusal path.
2. The real wiring is tested in a subprocess: the test bundles src/cli/bin.ts with esbuild (a dev dependency) into a temp file and spawns it with an environment built from scratch (no markers, so the test cannot pass because the runner is itself an agent session) and stdin a pipe carrying the account names, then with stdin ignored: exit 1, the "not a terminal" reason and no marker named, no allowed.yaml. If bin.ts treated a pipe as a terminal, the piped names would be accepted and the test fails.
3. A source scan: only src/cli/terminal.ts mentions process.stdin, node:readline, node:tty or isatty, so in-process tests run the same code the real CLI runs.
4. A manual check in the PR notes: the owner runs alz account allow once in a terminal window (accepts) and once in an IDE terminal with Claude Code (refuses).

test/acceptance/32-account-allow.test.ts, fixture test/fixtures/32-account-allow/ (new rather than reusing 31, which a second root and a moved source would change): personal/, hub/layer (kind client, private, account claude: client-a, codex: client-a-api, two names so two prompts), hub/docs, repo/ (personal, hub/layer at v1, .layer), repo-b/ (a second root). The hub becomes a git repo tagged v1, v2 (same account, other content) and v3 (another map) at test time, through account-workspace.ts with a fixture parameter. Fake HOME and XDG_*, env from scratch.
AC #1:
- No terminal: allow and --remove exit 1, nothing appears under HOME (listing before and after), and the trace shows no file system call at all.
- Each of the six markers, set to "" and to a unique marker string, with a terminal: allow and --remove refuse, name the marker, never show the string, never call ask, write nothing (24 cases). CLAUDECODE adds the IDE sentence.
- --yes, -y and a positional: exit 2, nothing written.
- The prompt shows root, accounts per target with folder existence, layer, source, ref, the v1 commit SHA and the recorded source. y, yes, empty, a wrong name, end of input, or a wrong second answer: nothing written. The exact names (one with a trailing space) are accepted.
- The file: exact bytes (header, sorted), mode 0600, created directory 0700 (parents created 0700), no temp or lock file left. The same-root entry is replaced; other entries, one with a root that does not resolve included, are kept and sorted. Running from a symlinked path or a subdirectory records the real root.
- A linked allowed.yaml stays a link and its target holds the entry; a dangling link refuses.
- Unsafe states, each exit 1 with the reason, the state directory and any link target byte-identical, no lock or temp file left, and no prompt when known before it: file writable by group, by others; directory, and a parent below HOME, writable by group; XDG_STATE_HOME inside the repo (nothing created in the repo); XDG_STATE_HOME moved with __MISE_DIFF, and with DIRENV_DIFF; XDG_STATE_HOME under ~/.claude-x (nothing created there); a folder or a FIFO in place of the file; over 1 MiB; invalid YAML; a mapping at the top; an ignored entry (relative root, bad name, non-real root, two entries for one root); relative HOME; uid undefined (writer-level test). Ownership by another user is not tested (needs root), as in test 31.
- Cache miss (lock missing, content hash differs, XDG_CACHE_HOME moved under mise): the message says only to run alz build, no prompt, nothing written. No declaration (with and without an entry: the --remove hint), a conflict (cache-built like test 31's remoteStack), an unreadable manifest, alz-blocked, and no repository / account folder / configured folder each refuse.
- Already allowed: exit 0, no prompt, the file's bytes and inode unchanged.
- The declaration changes while the prompt waits (the fake terminal's ask callback switches the stack to v3 and runs alz build in-process): refused as changed while confirming, nothing written.
- Lock: a lock file removed by a timer after 200 ms is waited for, then allow succeeds; at writer level with waitMs 200 a lock that stays refuses, naming it, and is left in place; a lock that is a link is not followed and its target is unchanged.
- Two concurrent runs, real processes: an esbuild-bundled driver calls updateAllowed from 8 processes for 8 roots and 4 processes for one shared root, started together: 9 entries, sorted, one per root, the shared root's entry equal to one of the four, no temp or lock file left, the file reads back clean. (In-process runs cannot overlap inside the synchronous critical section, so they would prove nothing about the lock.)
- --remove: removes only this root's entry, the others byte-equivalent and sorted; the last entry leaves "[]"; a missing file or no entry: exit 0, nothing created; works on a cache miss and with an unreadable manifest; asks for the recorded names.
- Trace (test/fs-trace.ts) of a successful allow: every write-type call (open with write flags, write, rename, mkdir, unlink, chmod) is under the real state directory or its mkdir chain; no child_process; nothing opened in a planted account folder; positive control: the trace sees renameSync.
- Import graph: account-allow.ts and account-allow-writer.ts import no git, tree cache, sources, pipeline or child_process; account-resolution.ts and account.ts (which) do not import the writer (also added to test 31's forbidden list); account-allow-hint.ts is imported only by build.ts and check.ts.
AC #2:
- Fresh clone: build; which reports default and "not allowed". build and check on a terminal print the hint on stderr; with each of the six markers, or without a terminal, they print nothing about it; stdout and the generated files are identical either way.
- After allow, which reports claude: client-a and codex: client-a-api from the layer, and build and check print no hint. A ref bump to v2 and a rebuild keep it.
- A changed map (v3), a target removed (claude only), and a changed source (the stack points at a copy of the hub) each make which fail closed with old and new values, and build and check print the hint with both; allow again makes it apply.
- Declaration removed while allowed: the --remove hint. allowed.yaml writable by group: the hint names the reason.
- The recorded entry: the real root, the whole map (a claude-only declaration records claude only), and the source: a local layer's real path, and a git: remote URL without its ref (cache-built, as in test 31).
- The hook and alz run are not built (TASK-50.6, TASK-50.3); here the import-graph test shows only build and check use the hint, and their own tasks test that they never print it.
test/acceptance/24-untrusted-repo.test.ts (account cases): a repository whose .layer declares an account: allow refuses ("the layers name no account here") and warns that the layer is ignored; an external client layer named by the manifest: allow shows its source quoted before the prompt; a hub directory whose name holds ESC: no control character in stdout or stderr.
AC #3: mise run alz:build, then mise run alz:check, pass; the generated AGENTS.md carries the new invariant.

### Threat-model checks that apply
1 Paths: the only writes outside the repo are in account-allow-writer.ts, to a path built from HOME, XDG_STATE_HOME and fixed names; never inside the repo or an account folder; trace test. 2 Subprocesses: allow runs none (import graph). 3 Locks: layer.lock only through the cached reader, sha validated. 4 Secrets: no secret is stored; marker and mise/direnv values are never read. 7 Symlinks: lock and temp O_NOFOLLOW|O_EXCL, a linked allowed.yaml kept and written at its real path, a dangling link refused. Also §9.1 (account folders by name only) and output quoting. 5 (security floor) and 6 (adoption) do not apply. A security-reviewer pass is required before In Review (label area:security).

### User-facing and version
New command, a new file written outside the repo, and a stderr note from build and check on a terminal. No change to the layer format, to generated output or to the lock format. It ships in the minor release that carries TASK-50 (held until every TASK-50 subtask is Done; TASK-50.2 notes), with release notes for the command, allowed.yaml and the hint.

### Verification
npx vitest run test/acceptance/32-account-allow.test.ts, then 24, 30 and 31; mise run test; mise run lint; mise run build; mise run alz:build; mise run alz:check; the manual terminal check above.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (@architect): plan recorded. Writer policy proposed as decision-20 (body filled under the owner's standing exception). Editorial spec fixes made at plan time: the tags '(TASK-50.2)' in LAYERING-SPEC §9.1, the §10 allow row and §13 test 20 now say TASK-50.11. Behavioural spec changes are listed in the plan under 'Spec changes in the same PR'. Owner questions 1 to 5 in the comment; label needs:owner-approval added.

2026-11-22 owner accepted all five defaults: (1) --remove also asks for the typed account name; (2) allow refuses and changes nothing when allowed.yaml has an entry the reader ignores; (3) one confirmation per distinct account name; (4) the threat-model skill also gets the checklist and environment-row changes; (5) alz run names alz account allow only as the fix for a changed declaration. decision-20 accepted.

2026-11-22 (@implementer): built per the plan and decision-20. New: src/cli/terminal.ts (only reader of stdin), src/cli/commands/account-allow.ts, src/resolve/account-allow-writer.ts (only writer outside repo and cache), src/resolve/account-allow-hint.ts (imported by build and check only). Changed: core/env.ts (markers), cli/io.ts, bin.ts, main.ts, build.ts, check.ts, account-resolution.ts (readRepositoryDeclaration extracted, behaviour unchanged), account-declaration.ts (declared.ref), account-allow-state.ts (shape, ignored, unresolved, serialize, loadAllowState, sameEntry), account-files.ts (unplacedReason and placementProblem shared by reader and writer). Agent layer: .layer invariant and threat-model rows, items 1 and 8; alz:build and alz:check pass. Spec: 7.13, 9.1, 10, 13 tests 20 and 24; README row and safety line. Tests: test/acceptance/32-account-allow.test.ts (110 cases, fixture 32-account-allow), cases in 24 and 31, test/allow-driver.ts and test/import-graph.ts (shared helpers). Deviations: updateAllowed is async (lock wait uses a timer so an in-process timer test can release it); the critical section after the lock is synchronous. The writer result carries no warnings; the command prints the reader's warnings itself. Open: the manual terminal check (accept in a terminal window, refuse in an IDE terminal with Claude Code) cannot be run by an agent; security-reviewer pass required before In Review.

2026-11-22 owner: allowed.yaml whose real path is inside any git work tree is refused by the reader and the writer (not only warned); the writer also refuses a link target whose content is not empty, [] or alz's own header. Coordinator: the Codex session-marker owner check (target-facts O5) and a new sandbox-launcher check (O8) are release gates for the TASK-50 minor release; §7.13 states that only Claude Code and Codex sessions are detected.

2026-11-22 (@implementer), review fixes: allowed.yaml inside any git work tree is ignored by the reader and refused by the writer (accounts.yaml only warns); a linked allowed.yaml is followed only to an empty, [] or alz-header file, and its target directory passes the same checks. alz escapes text a person decides on (quoteEscaped: \u{XXXX}); the prompt names root and source. Terminal: one shared line reader (createTerminal), queued lines, close on exit. Post-confirm check re-runs findRepositoryRoot. Lock: failed pid write closes and unlinks; release compares before closing. Hint: declares gate, stateProblem forms. alz account which lists stale approvals. Docs: spec 7.13, 9.1, 10, 13; README; skill rows and items; target-facts O8 and the item 20 NOTE; decision-20 amended. Tests: 32 now 150 cases, plus 24 and 31 additions; scan helpers cover dynamic import, require, side-effect imports, /dev/tty and fd 0 reads.

2026-11-22 (@implementer), final review fixes. For the release note: a ~/.git (or any .git above the state directory) makes every allowed.yaml below it ignored, so it withdraws every approval; only git checkouts are detected (not archives, jj without git, hg, sl or bare-repo dotfiles). A file reached through a link, or a linked folder above it, is refused before any prompt unless it is empty, [] or alz's own; the target is checked again just before the rename. Prompt names the root through quoteEscaped. Lock and temp files are named after the real file for a link. Not tested: the swap-before-rename check (the window is synchronous).
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-22T09:52:00Z

---

Owner questions for the plan (decision-20 is proposed). Each has a default; answering 'defaults' approves all five. Blocks implementation until answered; remove the label then.
1. --remove confirmation. The spec gives it only the terminal and agent-session checks. (a) Type the recorded account name or names, as allow does (recommended: removing sends client work back to the default login, so it deserves the same friction). (b) Checks only, no prompt. (c) Type 'remove'.
2. allowed.yaml with an entry the reader ignores (bad shape, relative root, invalid name, root not its real path, two entries for one root). (a) allow and --remove refuse and change nothing; the person fixes or deletes the file (recommended; matches 'plan first, change nothing' and decision-20). (b) Drop those entries and list them before the prompt (they have no effect today). A whole-file problem (owner, modes, location, invalid YAML, not a list) refuses either way.
3. A per-target map with two names (claude: client-a, codex: client-a-api). (a) One prompt per distinct name, in target order (recommended). (b) One prompt for all names separated by spaces.
4. Threat-model skill beyond the two trust rows: amend checklist item 1 (allowed.yaml is the exception to insideRoot), add item 8 (person-only commands: no TTY or any marker means refuse, no --yes, input only through Io.terminal), and qualify the environment row (markers prove presence only; XDG_* moved while mise or direnv is active is not trusted). Recommended: all three, since otherwise the skill contradicts the new invariant.
5. Spec contradiction: §7.13 says 'The shell hook and alz run never print [the allow command]', while §7.13 'fails closed' and §10 say alz run names alz account allow as the fix for a changed declaration or source. Proposed: the hint for a not-yet-allowed repository is never printed by the hook or alz run; alz run names allow only as its fail-closed fix. Affects TASK-50.3, not this task's code.

---
<!-- COMMENTS:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz account allow and --remove: the only command that writes outside the repository and the cache. Before any file-system call it refuses without a TTY on stdin or inside a Claude Code or Codex session (six markers, IDE terminals included); there is no --yes. It shows the account per target, the declaring layer, its source and revision (non-printable-ASCII escaped as \u{XXXX}), asks the person to type each distinct account name, re-reads the repository and declaration after the last answer, and writes allowed.yaml under a lock file through a temp file and rename (0600 in a 0700 folder, sorted, same-root entry replaced, read back), refusing every state the reader ignores, a file inside any git work tree, and a link target alz did not write (decision-20). --remove asks for the recorded names. build and check print the allow hint only on a terminal with no agent marker and only when a layer declares an account; check only when its comparison passed. alz account which lists stale approvals. The .layer invariant and the threat-model skill name allowed.yaml. Spec §7.13, §9.1, §10, §13; README. Tests: 32-account-allow (152), plus 24 and 31; 840 passed, 1 skipped; lint and alz:check clean. Reviewed in three rounds by reviewer and security-reviewer, both ready. Release gates: the owner's manual terminal check, O5 and O8.
<!-- SECTION:FINAL_SUMMARY:END -->
