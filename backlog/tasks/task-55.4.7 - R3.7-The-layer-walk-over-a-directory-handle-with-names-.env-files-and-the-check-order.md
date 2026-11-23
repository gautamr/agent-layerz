---
id: TASK-55.4.7
title: >-
  R3.7: The layer walk over a directory handle, with names, .env files and the
  check order
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T08:35:00Z'
labels:
  - area:rust
  - area:resolve
milestone: m-7
dependencies:
  - TASK-55.4.6
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 108390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Fixes TASK-69 (AC #2 and AC #4) for directories.
- alz-io `fs/scan.rs`: `Dir::scan(Purpose) -> LayerScan`:
  - openat recursion;
  - each name is an `OsStr`, tested with `to_str`; an invalid name is recorded, never printed;
  - `.git`, `.DS_Store` and `node_modules` are never entered;
  - links, FIFOs and devices are listed but not read;
  - a `.env` name is recorded and never opened;
  - regular files are read with O_NONBLOCK and fstat S_ISREG, and their sha256 is taken (sha2, decision-34);
  - bytes are counted as they are read.
  `Purpose::Resolution` stops at entry 2001 and counts 16 MiB of bytes; `Purpose::Build` has no budget.
- alz-core `layer/scan.rs`: `scan_verdict(Purpose, LayerScan)` applies §5.3's order.
  - Resolution: too many entries (unreadable), an invalid name (layer-invalid), unreadable or over 16 MiB (unreadable), then the content hash, then `.env` (layer-invalid).
  - Build: the name error first, then integrity (R3.8 and R3.9), then `env-file`.
  The verdict gives `LayerFiles`, so the content hash is computed only over names it has validated.
- Tests:
  - a FIFO, a device and a folder in place of a file, without blocking;
  - a file that grows past the cap;
  - the order of checks for any order of names (proptest);
  - a name with the byte 0xFF, made where the file system allows it and skipped on APFS.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A file or folder named with the byte 0xFF makes the scan report the §5.3 File names error naming the layer by its source, never the name (Rust test, skipped where the file system refuses the name; TASK-69 AC #1 in Rust)
- [x] #2 In resolution, too many entries wins over an invalid name, which wins over the 16 MiB budget, the content hash and .env; in the build the name error comes before integrity and env-file, for any order of names (proptest; TASK-69 AC #2 and AC #4)
- [x] #3 No .env file is opened, links and non-regular entries are not read, and a FIFO or device does not block the walk (Rust tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R3.6 (2026-11-23, lead): the caller resolves HOME, CLAUDE_CONFIG_DIR and CODEX_HOME to real paths once and passes them to AccountFolders::with_real_home / with_real_configured, and builds RepositoryPaths::new(as_written, real) for CacheDir::open_or_create.

Implementer (R3.7). Files: alz-core layer/scan.rs, layer/scan/verdict.rs, layer/scan/tests.rs (LayerScan facts, Purpose/Limits 2000 entries and 16 MiB, NameProblem, scan_verdict -> ReadLayer or LayerRefusal in check order TooManyEntries < NameNotUtf8 < NameNotSafe < Unreadable < EnvFile, LayerRefusal::miss() and file_name_diagnostic()); alz-io fs/scan.rs (Dir::scan(Purpose) -> LayerScan, infallible: every failure is a recorded fact; openat/O_NOFOLLOW per folder, lazy rustix Dir listing so it stops at entry 2001, explicit stack for depth, files opened O_NONBLOCK + fstat S_ISREG, never a .env), fs/dir.rs (scan method), read_below.rs (require_regular now pub(super)); tests alz-io/tests/layer_scan.rs. Digest taken with alz-core FileDigest::of (no new dependency in alz-io). AC#1: Linux-only tests (cfg target_os=linux, APFS refuses the 0xFF name with EILSEQ, observed) assert the exact spec message naming 'path:../hub' with no U+FFFD or 0xFF; NOT run here (macOS); compiled and clippy-checked by temporarily widening the cfg, then failed at EILSEQ on file creation as expected, reverted. The pure verdict is unit-tested for a not-UTF-8 name (core test a_layer_with_a_name_that_is_not_valid_utf8_is_a_cache_miss_layer_invalid_in_resolution). AC#2: proptests the_resolution_verdict_is_the_first_failing_check_for_any_order_of_names, the_verdict_does_not_depend_on_the_order_the_facts_arrive_in, in_the_build_a_name_error_precedes_every_other_refusal; real-tree twins too_many_entries_win_over_a_bad_name_... and after_the_budget_is_used_up_names_are_still_checked. TASK-69 AC#2 and #4 (directories): covered by those. AC#3: a_env_file_at_any_depth_is_recorded_and_never_opened (mode 000 files), a_link_is_counted_and_never_followed_or_hashed, a_fifo_and_a_socket_are_counted_and_do_not_block_the_walk (20 s timeout thread); a device node cannot be made unprivileged, FIFO and socket stand for non-regular entries. Limits: entry_2000/2001, the_walk_never_goes_past_entry_2001, 16 MiB exact and +1 (sparse files), env files and links add nothing, skipped names uncounted and unopened. Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, msrv, deny, audit), mise run lint exit 0. OPEN: (1) a name that is valid Unicode but no SafeRelPath (backslash, .git in another case, .git.) cannot be hashed; spec 5.3 is silent, so it is NameProblem::NotSafe / LayerRefusal::NameNotSafe, ordered right after the UTF-8 error, miss layer-invalid, with no build message yet - needs an architect decision on wording (R3.8/R3.10). (2) Build messages for Unreadable are unspecified; file_name_diagnostic returns None for them. (3) The env-file diagnostic is not built here (ReadLayer::env_files() gives the paths); R3.10 does it. (4) Reading the 0xFF name path on Linux unverified until CI.

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (516 tests; layer_scan 19), lint passes, src/ unchanged. AC #1's 0xFF-name tests are Linux-only (APFS refuses the name); AC #1 is checked once the draft R3 PR's ubuntu CI job runs them green. Lead: NameNotSafe (valid Unicode but not a SafeRelPath, e.g. a backslash or .GIT) stays a layer-invalid miss right after NameNotUtf8; R3.11 has the architect word its build message and the Unreadable one in the spec.
<!-- SECTION:NOTES:END -->
