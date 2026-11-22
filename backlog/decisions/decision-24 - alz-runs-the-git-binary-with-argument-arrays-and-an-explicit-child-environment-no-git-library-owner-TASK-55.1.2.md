---
id: decision-24
title: >-
  alz runs the git binary with argument arrays and an explicit child
  environment; no git library (owner, TASK-55.1.2)
date: '2026-11-22T16:27:00Z'
status: accepted
---
## Context

This is doc-5 choice 3. The TypeScript reference runs `git` through `execFileSync` with argument
arrays for:
- identity: `rev-parse --git-dir`, `var`, `config --show-scope --show-origin`;
- source resolution: `rev-parse`, `clone`, `checkout`;
- the pinned-layer tree cache: `ls-tree -z`, `cat-file --batch-check` and `--batch`;
- `init`: `tag --list`.

Each call runs in a guarded environment:
- `GIT_TERMINAL_PROMPT=0` on every call;
- a clone of a source classed as remote adds `-c protocol.file.allow=never` and drops
  `GIT_ALLOW_PROTOCOL`;
- every call that reads a tree sets `GIT_NO_REPLACE_OBJECTS=1` and `GIT_NO_LAZY_FETCH=1`.

target-facts §11 records the git behaviour these rely on (git 2.50.1), and spec §6 and §9.4
are written against it. Alternatives considered:
- gix (gitoxide) for local reads, with the git binary for the rest. That makes two git
  implementations with different semantics for replace refs, promisor fetches, config includes
  and `safe.directory`. gix is a large 0.x tree that releases about monthly, has MSRV 1.88 and a
  history of advisories.
- git2 (libgit2). It is C behind FFI, which makes static builds for four targets harder. Its
  config and credential behaviour differs from the git CLI, and it had unsoundness advisories in
  2026.

## Decision

The Rust port runs the system `git` binary for every git operation, through one `GitRunner` in
`alz-io`:
- `std::process::Command` with argument arrays, never a shell;
- untrusted operands placed so that git cannot read them as options: a revision follows
  `--end-of-options` (`rev-parse --verify --quiet --end-of-options <rev>`,
  `checkout --quiet --end-of-options <sha>`), and `--` comes only before paths, never before a
  ref, because git then reads the ref as a path; refs and SHAs are validated before use;
- the child environment built explicitly: alz clears the inherited environment and passes exactly
  the variables that one pure function of the `Env` snapshot and the kind of operation returns,
  the guard variables above included, so nothing reaches git that this function did not return;
- standard input null unless a batch command feeds it;
- machine-readable output (`-z`, porcelain formats) parsed into typed values (`ObjectId`, tree
  entries);
- the exit status checked, and stderr sanitised before it is shown.

Blob ids for cache verification are computed in Rust, not by git. No git library (gix, git2) is a
dependency. Code that must never run git, such as account resolution and the shell hook, does not
receive a `GitRunner`.

**Amended 2026-11-22 (owner, R0 review).** Two points of the list above changed.
- The child environment. The first text said it was built explicitly, yet left open whether the
  child keeps "inheriting" other `GIT_*` variables, which contradicted it. It is built explicitly,
  and for cache and clone operations (`clone`, and `rev-parse`, `checkout`, `ls-tree` and
  `cat-file` in a temporary clone or the cache) the R3 default removes the variables that point
  git at another repository: `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`,
  `GIT_OBJECT_DIRECTORY`, `GIT_ALTERNATE_OBJECT_DIRECTORIES`, `GIT_COMMON_DIR`, `GIT_NAMESPACE`
  and `GIT_CEILING_DIRECTORIES`. The case it prevents is `alz check` run as a pre-commit hook:
  git gives the hook `GIT_INDEX_FILE`, and `GIT_DIR` in a linked worktree, so a checkout in a
  temporary clone would write the user's index and a cache read would read the user's
  repository.
- Option termination. `--` before a revision makes git read it as a path, so revisions follow
  `--end-of-options`.

Evidence, git 2.50.1 in a scratch repository with a fake `HOME` (SMOKE):
- A pre-commit hook received `GIT_INDEX_FILE` on `commit`, `commit -a` and `commit -- <path>`, and
  `GIT_DIR` as well in a linked worktree. It also received `GIT_AUTHOR_NAME`,
  `GIT_AUTHOR_EMAIL`, `GIT_AUTHOR_DATE`, `GIT_PREFIX`, `GIT_EDITOR` and `GIT_EXEC_PATH`, and
  `GIT_CONFIG_PARAMETERS` when the commit ran with `git -c`.
- `rev-parse --verify --quiet --end-of-options 'v1^{commit}'` resolved the tag;
  `--end-of-options '--output=x'` exited 1 and wrote no file; `rev-parse -- v1` printed `--` and
  `v1` unresolved.
- `checkout --quiet --end-of-options <sha>` detached at the commit; `checkout --quiet -- <sha>`
  failed with "pathspec … did not match any file(s)".
- `--end-of-options` reached the parse-options commands, `checkout` among them, in git 2.24, and
  `rev-parse` in 2.30 (DOCS: git release notes 2.24.0 and 2.30.0), both below the spec's floors.

## Consequences

- Behaviour, target-facts §11 and the threat model's subprocess rules carry over unchanged, except
  inside a git hook (amendment below). The
  user's credential helpers, ssh configuration and URL rewrites keep working for private layers.
- The git version floors in the spec stay: 2.26 for `--show-scope`, 2.44 for
  `GIT_NO_LAZY_FETCH`, 2.45.1 for the local-clone fixes.
- Each git call starts a process. That happens on build paths only, never on the hook path.
- Computing blob ids needs a SHA-1 and SHA-256 crate, which is its own dependency decision under
  the doc-5 supply-chain rule.
- **Amended 2026-11-22 (owner, R0 review)**, replacing the open question on inherited `GIT_*`
  variables:
  - R3 (TASK-55.4) writes the environment function with the removals above for cache and clone
    operations, states them in spec §6 in the same pull request, and tests them with a hook
    fixture: `alz check` as a pre-commit hook, in a main checkout and in a linked worktree,
    leaves the user's index and repository byte-identical.
  - The TypeScript reference strips `GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR`, `GIT_CONFIG`
    and `GIT_INDEX_FILE` for identity calls only (`src/identity/inspect.ts`); its clone and
    checkout inherit the whole environment. Inside a hook the Rust binary therefore differs
    from it, and the spec change says so.
  - Still open for R3, and a spec change if adopted: `GIT_CONFIG_PARAMETERS` and
    `GIT_CONFIG_COUNT` with `GIT_CONFIG_KEY_<n>` and `GIT_CONFIG_VALUE_<n>`, which carry a parent
    git's `-c` options. They may hold settings a private clone needs, or settings that weaken a
    guard; which `-c` wins when both alz and the parent set one is UNVERIFIED.
  - Calls on the repository being built (identity, `init`'s `tag --list`) are not cache or clone
    operations. R6 (TASK-55.7) states their environment, including `GIT_AUTHOR_*` and
    `GIT_COMMITTER_*`, which a hook receives.
