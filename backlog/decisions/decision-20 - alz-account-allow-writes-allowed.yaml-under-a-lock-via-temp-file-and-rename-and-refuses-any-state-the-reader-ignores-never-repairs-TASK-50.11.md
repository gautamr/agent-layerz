---
id: decision-20
title: >-
  alz account allow writes allowed.yaml under a lock via temp file and rename,
  and refuses any state the reader ignores; never repairs (TASK-50.11)
date: '2026-11-22T09:51:00Z'
status: accepted
---
## Context

`allowed.yaml` is the only file alz writes outside the repository and the layer cache
(LAYERING-SPEC §7.13 'Allow state', §9.1). The reader built in TASK-50.10 ignores the file, or
single entries, in many states: wrong owner or modes, inside the repository or an account
folder, located by an `XDG_STATE_HOME` that mise or direnv may have moved, not a regular file,
over 1 MiB, invalid YAML, entries with a bad shape, a root that is not its real path, or two
entries for one root. A writer that wrote anyway would either store approvals the reader then
ignores, or overwrite content alz did not write. Two `alz account allow` runs can also race.

## Decision

- Only `alz account allow` and `alz account allow --remove` write `allowed.yaml`, through one
  module (`src/resolve/account-allow-writer.ts`).
- They hold an exclusive lock file next to the file that is replaced (`O_CREAT|O_EXCL|O_NOFOLLOW`),
  wait up to 10 seconds for it, and never remove a lock they did not create; a stale lock is
  reported for the person to delete.
- Under the lock they read the file again with the reader itself, and refuse, changing nothing,
  in every state the reader ignores, for the file or for any entry, and also for a dangling
  link, a result over 1 MiB, or a result that does not parse back to the same entries.
- They write through a temporary file in the same directory (`O_EXCL|O_NOFOLLOW`, mode 0600),
  fsync and rename. A linked `allowed.yaml` stays a link and its real path is replaced. Missing
  directories are created with mode 0700; existing ones keep their mode and must pass the
  reader's checks.
- Entries are sorted by root; the entry for the same real root is replaced; valid entries whose
  root no longer resolves are kept.
- They never repair: no chmod of an existing file or directory, no dropping of entries, no
  rewrite of content they cannot parse. The person fixes or deletes the file.

## Consequences

The reader and the writer cannot disagree about which file or entry counts. A hand-edited or
corrupted `allowed.yaml` blocks `allow` and `--remove` until the person fixes or deletes it
(deleting withdraws every approval). The owner answered owner question 2 of TASK-50.11 with
option (a): entry-level problems refuse too, rather than being dropped and listed. Applied in
TASK-50.11 with LAYERING-SPEC §7.13 and §9.1.

## Amendment (2026-11-22, owner, after review)

- The reader and the writer refuse an `allowed.yaml` whose real path lies inside any git work
  tree (found by `lstat` of `.git`; no git runs), because a checkout could plant it.
- A linked `allowed.yaml` is followed only to a file that is empty, holds `[]` or starts with
  the header alz writes; the directory of its real path passes the same checks as the
  directory of the link. Missing parent folders are created with mode 0700.
