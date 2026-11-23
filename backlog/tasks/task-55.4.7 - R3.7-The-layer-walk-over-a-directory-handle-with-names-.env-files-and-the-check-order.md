---
id: TASK-55.4.7
title: >-
  R3.7: The layer walk over a directory handle, with names, .env files and the
  check order
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T08:27:00Z'
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
- [ ] #2 In resolution, too many entries wins over an invalid name, which wins over the 16 MiB budget, the content hash and .env; in the build the name error comes before integrity and env-file, for any order of names (proptest; TASK-69 AC #2 and AC #4)
- [ ] #3 No .env file is opened, links and non-regular entries are not read, and a FIFO or device does not block the walk (Rust tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R3.6 (2026-11-23, lead): the caller resolves HOME, CLAUDE_CONFIG_DIR and CODEX_HOME to real paths once and passes them to AccountFolders::with_real_home / with_real_configured, and builds RepositoryPaths::new(as_written, real) for CacheDir::open_or_create.
<!-- SECTION:NOTES:END -->
