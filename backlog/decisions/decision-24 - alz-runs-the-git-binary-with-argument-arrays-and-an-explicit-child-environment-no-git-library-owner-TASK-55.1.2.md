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
- `--` before untrusted positionals, and refs and SHAs validated before use;
- the child environment built explicitly from the `Env` snapshot, with the guard variables above
  for each kind of operation;
- standard input null unless a batch command feeds it;
- machine-readable output (`-z`, porcelain formats) parsed into typed values (`ObjectId`, tree
  entries);
- the exit status checked, and stderr sanitised before it is shown.

Blob ids for cache verification are computed in Rust, not by git. No git library (gix, git2) is a
dependency. Code that must never run git, such as account resolution and the shell hook, does not
receive a `GitRunner`.

## Consequences

- Behaviour, target-facts §11 and the threat model's subprocess rules carry over unchanged. The
  user's credential helpers, ssh configuration and URL rewrites keep working for private layers.
- The git version floors in the spec stay: 2.26 for `--show-scope`, 2.44 for
  `GIT_NO_LAZY_FETCH`, 2.45.1 for the local-clone fixes.
- Each git call starts a process. That happens on build paths only, never on the hook path.
- Computing blob ids needs a SHA-1 and SHA-256 crate, which is its own dependency decision under
  the doc-5 supply-chain rule.
- Open for R3 (TASK-55.4), and a spec change if adopted: whether the child should stop inheriting
  other `GIT_*` variables, such as `GIT_DIR` and `GIT_CONFIG_PARAMETERS`.
