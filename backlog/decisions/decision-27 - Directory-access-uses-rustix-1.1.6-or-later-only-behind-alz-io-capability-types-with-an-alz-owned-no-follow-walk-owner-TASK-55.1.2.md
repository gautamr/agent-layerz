---
id: decision-27
title: >-
  Directory access uses rustix 1.1.6 or later only, behind alz-io capability
  types with an alz-owned no-follow walk (owner, TASK-55.1.2)
date: '2026-11-22T16:27:00Z'
status: accepted
---
## Context

doc-5 asks for file-system safety from a safe crate chosen in R0, and its principle 6 asks to close
the Node limits: `openat`-relative access, and the component-by-component link walk of TASK-52 and
TASK-53. What alz needs:
- reads relative to a directory handle with `O_NOFOLLOW`, for layers and cache entries;
- for account resolution, a walk one component at a time with `lstat` and `readlinkat` that checks
  each resolved prefix against the account folders. It crosses absolute links, so it is not a
  "beneath" sandbox;
- lock and temporary files created with `O_CREAT|O_EXCL|O_NOFOLLOW` and mode `0600`;
- `mkdirat` with mode `0700`, and `renameat`;
- `fstat` of owner and mode, and `getuid`;
- later, `symlinkat` for `alz account init`.

`std::fs::Dir` is still unstable in the 1.100.0 standard library (feature `dirfd`, issue 120426),
and std has no safe `getuid`. Candidates checked on 2026-11-22:
- rustix 1.1.6 (2026-10-27): a Bytecode Alliance project with MSRV 1.65. It uses a libc backend
  on macOS and `linux_raw` on Linux, and tempfile already depends on it. RUSTSEC-2026-0486
  covers unsound xattr functions below 1.1.6.
- cap-std 4.0.3 with cap-fs-ext and cap-tempfile:
  - On macOS, and on Linux without `openat2`, its sandbox runs in user space, and it had a sandbox
    escape until 4.0.3 (RUSTSEC-2026-0482, 2026-08-20: a trailing slash defeated `O_NOFOLLOW`).
  - By default it follows links that stay inside the sandbox.
  - It adds about nine crates.
  - On 2026-10-15 the project left the Bytecode Alliance for its main maintainer's own account.
  - alz would still need rustix for the account walk and `getuid`.
- nix 0.32.0: a broader API with more breaking releases, and it duplicates rustix.

## Decision

rustix, version 1.1.6 or later, is the only crate for low-level file-system and process-identity
calls. alz enables the features `fs` and `process`, and others only when a task needs them. It is
called only inside `alz-io`, wrapped in capability types (`ReadOnlyDir`, `RepoRoot`, `CacheDir`,
`StateDir`; doc-6 §3.4).

Their methods take validated relative paths (`SafeRelPath`) and walk them one component at a time
relative to a directory handle:
- each component is opened with `O_NOFOLLOW`, `O_DIRECTORY` and `O_CLOEXEC`;
- metadata comes from `statat` with `AT_SYMLINK_NOFOLLOW`, or from `fstat` on the open handle;
- a link is followed only where the spec allows it, explicitly, with `readlinkat`, and each
  resolved prefix is checked (account folders).

Files are created with `O_CREAT|O_EXCL|O_NOFOLLOW` and an explicit mode. A file is replaced through
a temporary file in the same directory, `fsync` and `renameat`. Neither cap-std nor nix is a
dependency. If `std::fs::Dir` becomes stable, replacing rustix calls with it is a new decision.

## Consequences

- `unsafe_code = "forbid"` still holds in every alz crate. rustix's own unsafe code stays inside
  the dependency.
- The walk in `alz-io` is security-critical code with its own tests:
  - links planted in the repository, in layers and in cache entries;
  - trailing slashes, the class of bug behind RUSTSEC-2026-0482;
  - case and Unicode variants;
  - account folders reached through links;
  - a trace that shows no call under an account folder.
- `cargo deny` (TASK-55.1.4) requires rustix 1.1.6 or later.
- TASK-52 and TASK-53 are closed in R3 (TASK-55.4) with these types, and the spec's Limit text is
  narrowed there.
- `openat2` on Linux and `O_NOFOLLOW_ANY` on macOS may be added as extra hardening, but are not
  relied on.
- Open for R5 (TASK-55.6), and a spec change if adopted: whether writes through links inside the
  repository, which the TypeScript writer follows today, are refused.
