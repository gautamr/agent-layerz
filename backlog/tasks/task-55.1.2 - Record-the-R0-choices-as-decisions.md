---
id: TASK-55.1.2
title: Record the R0 choices as decisions
status: In Progress
assignee: []
created_date: '2026-11-22T15:52:00Z'
updated_date: '2026-11-22T16:31:00Z'
labels:
  - area:rust
  - area:spec
milestone: m-7
dependencies:
  - TASK-55.1.1
references:
  - decision-23
  - decision-24
  - decision-25
  - decision-26
  - decision-27
parent_task_id: TASK-55.1
priority: high
ordinal: 60390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The owner decides; the architect drafts each with options and a recommendation, then fills the decision body after the owner answers (standing exception). Choices from doc-5: (1) repository layout and crate split (adopt or change doc-6); (2) the YAML library (from the spike); (3) git access: the git binary with argument arrays (recommended) or a library; (4) MSRV and target platforms; (5) message wording policy (the spec defines it; clap's conventions update the spec on purpose); (6) safe directory access crate (cap-std or rustix).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Six accepted decisions exist, one per choice, each with Context, Decision and Consequences filled
- [ ] #2 doc-5 and doc-6 are updated where a decision changes them
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 architect: drafts for choices 1, 3, 4, 5, 6 (choice 2 is the YAML spike, TASK-55.1.1). The owner decides; no decision records yet. Checked today: crates.io API, RustSec advisory-db, crate sources (rustix 1.1.6, cap-std 4.0.3), installed toolchains, and a scratch workspace built with rustc 1.99.1 and clap 4.6.9. Marked UNVERIFIED where not checked.

### Choice 1: repository layout and crate split
- 1a. Adopt doc-6 as written: workspace at the repo root next to the TypeScript code; crates alz-core (pure), alz-io (every effect), alz (clap binary), alz-test-support; xtask and fuzz in the skeleton.
- 1b. doc-6 with two changes: the alz binary also gets a clippy.toml banning std::fs, std::process::Command and std::env::var*, so only alz-io can reach an effect (doc-6 bans them in alz-core only); xtask and fuzz are added when their first task needs them, not in the skeleton.
- 1c. Two crates: alz-core plus alz with effects as modules. Less cross-crate API, but outside the core the effect boundary is a convention again.
- Verified (clippy 1.99): a crate's own clippy.toml replaces the root one, it is not merged. Every crate file must repeat the test allowances (allow-unwrap-in-tests and the others), or tests fail -D warnings. TASK-55.1.3 needs this.
- Recommendation: 1b. It is the gr-rust rule (split only for a boundary the compiler enforces), applied to both sides of alz-io.

### Choice 3: git access
- 3a. The git binary only. A GitRunner in alz-io passes argument arrays with -- before untrusted positionals and sets the child environment explicitly, as today: GIT_TERMINAL_PROMPT=0; guarded clone adds -c protocol.file.allow=never and drops GIT_ALLOW_PROTOCOL; tree reads set GIT_NO_REPLACE_OBJECTS=1 and GIT_NO_LAZY_FETCH=1. Output is read with -z and parsed into typed values (ObjectId). Blob ids are computed in Rust. Account resolution gets no GitRunner.
- 3b. gix (gitoxide) for local reads (rev-parse, ls-tree, cat-file), with the git binary for clone and identity. That means two git implementations with different semantics (replace refs, promisor fetch, config includes, safe.directory). gix is a large, fast-moving 0.x tree (0.90.0, 2026-10-22, releases about monthly, MSRV 1.88) with a history of advisories (gix-path, gix-transport, gix-fs; gix-features RUSTSEC-2025-0021, SHA-1 collision detection). All target-facts §11 rows would need checking again.
- 3c. git2 (libgit2): a C library over FFI. Vendored C (plus OpenSSL or libssh2 for remotes) makes static builds for four targets harder. Its config and credential behaviour differs from the git CLI, and it had 2026 unsoundness advisories (RUSTSEC-2026-0008, -0183, -0184; fixed in 0.21.0).
- Recommendation: 3a. Behaviour, target-facts §11 and the threat model stay as they are, and the user's credential helpers, ssh and URL rewrites keep working for private layers. Costs: a git floor (2.26 for --show-scope, 2.44 for GIT_NO_LAZY_FETCH, 2.45.1 for the local-clone CVEs; installed here: 2.50.1) and one process start per call, none of it on the hook path. Whether to clear inherited GIT_* variables (GIT_DIR, GIT_CONFIG_PARAMETERS) is a separate spec question for R3, out of scope here.

### Choice 4: MSRV and target platforms
- Installed: the active stable is 1.99.1; rustup check reports 1.100.0 (2026-11-09) as the current stable, and 1.100.0 and 1.91.0 are also installed. Rust comes from rustup, not mise (mise.toml has no rust entry). Edition 2024 needs 1.85 or later. resolver = "3" has to be set explicitly in a virtual workspace (from Cargo docs, UNVERIFIED here). doc-6 says mise reads rust-toolchain.toml: UNVERIFIED; rustup reads it either way.
- 4a. Pin rust-toolchain.toml to 1.100.0 (not "stable") and set rust-version = "1.100". Raise both together in one reviewed change; CI's main job is the MSRV job.
- 4b. Pin 1.100.0 for development and set the MSRV a few releases back (for example 1.97), with a second CI job on that toolchain. Resolver 3 then prefers compatible dependency versions.
- 4c. MSRV 1.85, the floor for edition 2024. Widest support, but it rules out later language and std features (let chains in edition 2024 came after 1.85: UNVERIFIED exact version).
- Targets, for all options: aarch64-apple-darwin, x86_64-apple-darwin (tier 2 since about 1.90: UNVERIFIED), x86_64-unknown-linux-musl and aarch64-unknown-linux-musl as static release binaries (any distro, Alpine). Glibc builds from source work too. Windows is still undecided and out of scope.
- Recommendation: 4a. alz ships prebuilt binaries and has no library consumers, so a lower MSRV buys little and costs a second CI toolchain.

### Choice 5: message wording policy
- The spec states specific messages and exit codes per command, but not the general output contract. The diagnostic line "<level> [<code>] <where>: <message>", exit 70 for an internal failure, --version printing "0.1.0", -v, and the usage text exist only in code and tests (25-cli, 27-install-local, 31-account-which, 32-account-allow).
- Verified (clap 4.6.9): --version prints "alz 0.1.0"; -V is the short form and -v is a usage error; no command prints help to stderr with exit 2; usage errors print "error: ...", "Usage: alz ...", exit 2. clap echoes raw arguments: on a pty, ESC sequences and U+202E reach the terminal; off a tty, anstream strips CSI sequences but keeps \r and U+202E. So alz must render clap errors itself (try_parse, then sanitise) to keep the stripControl rule (test 31 "removes control characters ... in the usage error").
- 5a. The spec owns the contract surface and clap owns help and usage. The spec gains an "Output and exit codes" section: streams, the diagnostic line format, exit codes 0/1/2/70, sanitising of all untrusted text including echoed arguments, and every message it already quotes. Help, usage, usage-error wording and --version ("alz <version>", -V) follow clap's defaults, and tests assert only exit code and stream for them. R1 rewrites the "usage: alz" and version assertions. The spec text is applied in the PR that builds the Rust CLI.
- 5b. Keep today's text byte for byte (the custom USAGE, -v, a bare version, Node parseArgs wording). That imitates the TypeScript output, which decision-22 rules out.
- 5c. The spec owns all text: clap only parses, and alz renders help, usage and errors from text in the spec. Stable, fully owned text, but more code and spec to maintain, and the CLI looks less like a standard Rust CLI.
- Recommendation: 5a.

### Choice 6: safe directory access
- What alz needs, from the spec and TASK-52/53: openat-relative reads with O_NOFOLLOW for layers and cache entries; a component-by-component walk with lstat and readlinkat for account resolution that checks each resolved prefix against account folders (this crosses absolute links, so it is not a "beneath" sandbox); O_CREAT|O_EXCL|O_NOFOLLOW lock and temp files with mode 0600; mkdirat 0700; renameat; fstat owner and mode; getuid; later symlinkat for account init. std::fs::Dir is still unstable (feature dirfd, issue 120426) in the 1.100.0 std source, and std has no safe getuid.
- 6a. rustix (features fs, process) as the only primitive, wrapped by alz-io capability types (ReadOnlyDir, RepoRoot, CacheDir, StateDir) with alz's own walk: SafeRelPath first, then each component opened with O_NOFOLLOW|O_DIRECTORY|O_CLOEXEC, and statat with AT_SYMLINK_NOFOLLOW. 1.1.6 (2026-10-27), Bytecode Alliance, MSRV 1.65, MIT/Apache; libc backend on macOS and linux_raw on Linux; renameat_with on apple and linux, O_NOFOLLOW_ANY on apple and openat2 on Linux (checked in its source). Already a transitive dependency of tempfile 3.27. RUSTSEC-2026-0486 (unsound xattr functions below 1.1.6, unused by alz): require 1.1.6 or later. Cost: alz writes and tests the walk itself, one module for the security review. It is simpler than a general resolver because alz never follows a link in layer or cache paths.
- 6b. cap-std 4.0.3 with cap-fs-ext and cap-tempfile for sandboxed repo, cache and layer access, plus rustix for the account walk, getuid and the allowed.yaml checks. On Linux 5.6 and later the kernel enforces RESOLVE_BENEATH. On macOS, and on Linux without openat2, its userspace resolver had a sandbox escape until 4.0.3 (RUSTSEC-2026-0482, 2026-08-20: a trailing slash defeated O_NOFOLLOW). By default it follows links that stay inside the sandbox, so alz would need the nofollow variants anyway. It adds about nine crates (cap-primitives, io-extras, io-lifetimes, ambient-authority, fs-set-times, ipnet, maybe-owned, rustix-linux-procfs, uuid), and on 2026-10-15 the project left the Bytecode Alliance for its main maintainer's own account. That makes two libraries for one concern.
- 6c. nix 0.32.0 (2026-11-14, MSRV 1.69, MIT). Also safe openat and the rest, but a broader API with more breaking 0.x releases, and it duplicates rustix, which is in the tree anyway.
- Recommendation: 6a. A separate spec question for R5, which this decision does not settle: today's writer follows links inside the repository (a .claude link, for example); Rust can refuse them, which would be a spec change.

2026-11-22 architect: the owner answered in this session: 1b, 3a, 4a, 5a, 6a. Decisions created as accepted, with Context, Decision and Consequences filled by hand under the standing exception:
- decision-23: Rust workspace per doc-6 (alz-core, alz-io, alz, alz-test-support); only alz-io reaches fs, process or env; xtask and fuzz added when first needed (choices 1 and 6 of doc-5's list)
- decision-24: the git binary with argument arrays and an explicit child environment; no git library (choice 3)
- decision-25: Rust 1.100.0 pinned through mise, MSRV equal to the pin; macOS arm64 and x86_64, static musl Linux x86_64 and arm64 (choice 4)
- decision-26: the spec owns streams, diagnostics and exit codes; clap owns help, usage errors and --version, sanitised by alz (choice 5)
- decision-27: rustix 1.1.6 or later only, behind alz-io capability types with an alz-owned no-follow walk (directory access)
doc-5 updated through backlog doc update: "Choices to settle" became "Choices settled in R0", pointing to the decisions with YAML still open (TASK-55.1.1); the Rust standard bullets name decision-25, -26 and -27; the R7 row lists the four targets. doc-6 updated: decisions in the header; in the §2 tree, the alz binary's clippy.toml, the note that a crate's clippy.toml replaces the root one, xtask and fuzz marked as later, rustix and the git binary named, rust-toolchain.toml kept only if it matches the mise pin; the §2 bullet on the binary ban. The spec is not touched: the "Output and exit codes" section is a consequence of decision-26 for a later task. Still open for AC #1: the YAML decision (choice 2, TASK-55.1.1). No acceptance criteria checked.
<!-- SECTION:NOTES:END -->
