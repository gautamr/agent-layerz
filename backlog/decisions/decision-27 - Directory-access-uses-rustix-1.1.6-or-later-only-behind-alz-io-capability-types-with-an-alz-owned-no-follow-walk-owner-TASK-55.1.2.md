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
- metadata comes from `statat` with `AT_SYMLINK_NOFOLLOW`, or from `fstat` on the open handle
  (amended below: every trust check uses `fstat`);
- a link is followed only where the spec allows it, explicitly, with `readlinkat`, and each
  resolved prefix is checked (account folders).

Files are created with `O_CREAT|O_EXCL|O_NOFOLLOW` and an explicit mode. A file is replaced through
a temporary file in the same directory, `fsync` and `renameat`. Neither cap-std nor nix is a
dependency. If `std::fs::Dir` becomes stable, replacing rustix calls with it is a new decision.

**Amended 2026-11-22 (owner, R0 review).** Checks are made on the handle that is used, never on a
name that is opened afterwards:
- Every trust check (owner, mode, file type, size) uses `fstat` on the handle that is then read,
  or, for a folder, on the handle the next step opens relative to. A `statat` followed by an
  `openat` of the same name is never a trust check, because the name can change in between.
  `statat` with `AT_SYMLINK_NOFOLLOW` may explain a refusal, for example name the link after an
  `O_NOFOLLOW` open failed, but never decides one.
- A file is opened with `O_RDONLY`, `O_NONBLOCK` and `O_CLOEXEC`, and with `O_NOFOLLOW` wherever
  the spec forbids a link, so a FIFO or a device cannot block the open. `fstat` must then show
  `S_ISREG`; anything else is refused.
- The 1 MiB cap is enforced by reading at most the cap plus one byte and refusing the file if
  that last byte arrives. `st_size` is not trusted. Budgets over many files, such as a layer's
  16 MiB, count the bytes actually read.
- A link is still followed only where the spec allows it, with `readlinkat` and a check of each
  resolved prefix.

**Amended 2026-11-24 (owner, R6).** rustix gains two features, each for one task:
- `termios`, in `alz-io`, for the confirmation prompt of `alz account allow` (TASK-55.7.11):
  `isatty` for the terminal check, and reading and setting the terminal's settings so that, while
  an answer is read, the interrupt key sends no signal and ends the line, and Ctrl-C answers no
  (spec §7.13 "Trust"). Line editing and echo stay on, and the saved settings are restored on
  every path.
- `pty`, dev only, in `alz-test-support`, for the pseudo-terminal driver that runs `alz account
  allow` in tests (TASK-55.7.1). `alz-test-support` is only ever a dev-dependency, so the `alz`
  binary is built without this feature.
  The driver also enables `termios` there, to hand the terminal to the child's process group
  (`tcsetpgrp`); it is dev only as well (lead, 2026-11-24, within the owner's pty choice).

`alz-test-support` is thus the one other alz crate that may depend on rustix directly. The rule
that only `alz-io` calls rustix still holds for every crate the binary is built from.

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
- **Amended 2026-11-22 (owner, R0 review):**
  - The default for that R5 question is to refuse writes through links inside the repository,
    unless the spec decides otherwise. Refusing changes what users see, for example a generated
    file the user replaced with a link, so R5 puts it in the spec and in the cutover's version
    note.
  - The walk's tests also cover a FIFO, a folder and a device in place of a file, which must be
    refused without blocking; a file that grows past the cap between `fstat` and the read; and a
    name swapped for a link between two calls, which must not change what was checked.
- **Amended 2026-11-24 (owner, R6):**
  - `deny.toml`: the rustix entry's `wrappers` gains `alz-test-support` next to `alz-io` and
    `tempfile`, with the reason "decision-27: the pseudo-terminal driver, dev only", in the change
    that adds the driver (TASK-55.7.1). No other crate is added.
  - `alz-io`'s rustix dependency gains `termios` (TASK-55.7.11); `alz-test-support`'s enables
    `pty` only. TASK-55.7.1 records, with `cargo tree`, that the release build of `alz` has neither
    `alz-test-support` nor the `pty` feature.
  - The prompt's tests, through the driver, cover the settings restored after an answer, after
    the end of input, after Ctrl-C and after an error, and a terminal whose settings cannot be
    read, which asks nothing.
