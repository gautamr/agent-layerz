---
id: TASK-55.4.8
title: >-
  R3.8: git: and github: sources: guarded clone, cache entries and lock
  integrity
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T08:47:00Z'
labels:
  - area:rust
  - area:resolve
milestone: m-7
dependencies:
  - TASK-55.4.3
  - TASK-55.4.4
  - TASK-55.4.7
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 109390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Fixes TASK-68 (AC #4 and the wiring of AC #1). It uses R3.3, R3.4 and R3.7.
- alz-core `stack/origin.rs`: `Origin::of(&StackEntry)`:
  - `github:` gives `https://github.com/<o>/<r>.git`, remote;
  - `git:` is classified sans-IO, in the order of target-facts §11 "Clone argument is tried as a local path first": git's suffixes looked up on disk first, a `..` segment refused for a local path, then `file://`, then `scheme://` or `host:path` as remote.
  The local lookup answers come from alz-io, and a canonical path through the walk (R3.6). The clone argument for a local source is that canonical path.
- `lock/integrity.rs`: `locked_entry_for` and `verify`.
- alz-io `resolve/clone.rs`:
  1. When the lock pins the source and its entry exists, check it against the lock's content hash before `layer.yaml`, and stop there.
  2. Otherwise clone into a `TempEntry`, run `RevParseCommit` of the lock's sha or the ref, and `Checkout`.
  3. If an entry exists, compare the content hashes, discard the clone, and give the cache error on a difference. If none exists, publish the clone.
  `rev` is `ObjectId::short_rev()`.
  Errors:
  - a failed git step: `cannot fetch` with `FetchFailureLine`;
  - after git, a name that is not UTF-8: the §5.3 error;
  - a read or rename failure: `cannot store …`.
- Integration tests with real git, using a local bare repository and a fake ssh through GIT_SSH for "remote" URLs:
  - a lock pin that hits the cache, and one that misses;
  - a moved ref;
  - an existing entry against a fresh clone;
  - a tampered entry;
  - an entry that is a link;
  - `x:y` as a local folder refused for a remote clone;
  - a failed clone, a failed rev-parse and a failed checkout;
  - a cache root made read-only before publish, giving `error [cache]`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A git: or github: layer is cloned once into git-<key>@<sha> through a temporary entry, read from the cache when the lock pins it, and recorded with rev equal to the 12-character sha prefix (integration tests with real git)
- [ ] #2 A cached entry that differs from the lock's content hash, or from a fresh clone when the lock has no entry, stops with the cache error naming the layer and its directory, and a link at the entry name is refused (integration tests)
- [ ] #3 A failed clone, rev-parse or checkout gives error [source] cannot fetch with the stripped first line, and a failure to read or publish the clone gives error [cache] cannot store (integration tests; TASK-68 AC #4)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R3.3 (2026-11-23, lead): git runs in its own session, so SIGINT to alz does not reach it and alz exiting does not stop a clone. Make the runner kill the call's process group when the call is abandoned (alz interrupted, or a guard dropped), with a test that interrupts alz during a slow clone and finds no git or ssh left.

Implementer (R3.8), uncommitted on feat/rust-r3-task-55.4. Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit); mise run lint exit 0. Not run: mise run test / ALZ_BIN acceptance (no TypeScript or acceptance file changed; spec text changed in §6 'No terminal').

Files: alz-core lock/integrity.rs (locked_sha_for, locked_entry_for, verify, Integrity), stack/origin.rs (CloneOrigin, Probe, OriginRefusal, github_origin, locate_git: sans-IO classification, git's suffix order, '..' refused for a local path, file:// with percent-decoding, 'scheme://' or 'host:path' remote), stack/git_ref.rs + lock/object_id.rs (ObjectId::short_rev, GitRef::of_commit), stack/resolution_failure.rs (cache_differs_from_clone, local_git_path_has_dotdot, invalid_file_url, layer_touches_account_folder). alz-io resolve/clone.rs (Fetcher, GitLayer, FetchedLayer, ReadHow), resolve/locate.rs (locate_git_source: the probe goes through the component walk, so an account-folder path is Refused before it is looked at), fs/dir.rs (Dir::open_with_real_path, Dir::contains), git/detach.rs and git/spawn.rs (abandoned-call kill). alz-test-support: ScratchDir helpers, start_silently. Tests: crates/alz/tests/git_sources.rs (20), crates/alz/tests/git_runner.rs (+1). Spec §6 'No terminal': new 'Abandoned calls' bullet.

AC #1: git_sources.rs a_git_layer_is_cloned_once_into_its_entry_with_the_twelve_character_rev (entry git-<key>@<sha>, rev = first 12 hex, temp entry gone, .env.dev listed and not hashed), a_layer_the_lock_pins_is_read_from_the_cache_without_cloning_again (source repo removed, ReadHow::Cached), a_pin_the_cache_lacks_is_cloned_at_the_pinned_commit_though_the_ref_moved, without_a_pin_a_moved_ref_gives_the_new_commit_in_a_new_entry, a_remote_source_is_cloned_through_ssh_and_its_entry_is_named_after_the_url (fake ssh via GIT_SSH_COMMAND).
AC #2: an_entry_that_differs_from_the_lock_stops_with_the_cache_error_before_any_git_call (whole message with layer name, source, directory), a_file_added_to_a_cached_entry_is_a_difference_too, an_existing_entry_is_compared_with_a_fresh_clone_when_the_lock_has_no_entry (equal: ok; tampered: 'differs from a fresh clone' error, entry untouched, clone discarded), an_entry_that_is_a_link_is_refused_with_and_without_a_lock (nothing written through the link, no .tmp- left).
AC #3 (TASK-68 AC #4): a_clone_that_fails_is_cannot_fetch_with_the_first_line_of_git_s_standard_error, what_a_remote_writes_to_standard_error_reaches_the_message_without_control_characters (ESC and CR removed; second line dropped), a_lock_sha_that_is_not_a_commit_is_cannot_fetch_without_a_line, a_ref_that_is_not_a_commit_is_cannot_fetch_from_the_rev_parse_step, a_checkout_that_fails_is_cannot_fetch_with_its_first_line (git wrapper on PATH), a_clone_past_its_deadline_is_cannot_fetch_and_leaves_no_temporary_entry ('git did not finish within 2 seconds'), a_cache_root_that_cannot_be_written_is_cannot_store_and_not_cannot_fetch, a_cache_root_made_read_only_before_the_publish_is_cannot_store, a_clone_that_cannot_be_read_is_cannot_store_and_not_cannot_fetch (layer.yaml made mode 000 after checkout).
Origin: a_local_git_source_is_cloned_from_its_canonical_path_and_a_found_folder_wins_over_a_url (x:y folder is Local; a link gives the canonical path; other:y is Remote; '..' refused), a_local_git_source_in_an_account_folder_is_refused_before_it_is_looked_at (direct and through a link in the repository). 12 core unit tests in origin.rs and integrity.rs.
R3.3 hand-off (abandoned call): interrupting_alz_during_a_slow_clone_leaves_no_git_or_ssh (git_runner.rs): the test binary runs a clone with a sleeping fake ssh and the standard deadline; the parent sends SIGINT; the test asserts both the ssh pid and the git pid are dead. Verified red first: with the parent check disabled the test fails ('ssh .. is alive after alz was interrupted'). Mechanism, still safe code: the copy no longer execs git. It is told alz's pid, runs setsid, spawns git as its child in that session and polls try_wait and getppid (1 ms backing off to 25 ms); when its parent changes it kills the process group with rustix kill_process_group, and otherwise ends with git's exit code (128+signal for a signal). spawn.rs gained a Running guard: a call dropped before it finished (error, panic) ends the group.

Open / for the lead:
1. Not tested: the §5.3 'name not valid UTF-8' result of a fetched clone (macOS refuses such names; the verdict path file_name_diagnostic is covered in alz-core). Add a Linux-only case in R3.11.
2. Not tested: 'x:y as a local folder refused for a remote clone' through the driver: it needs a folder in the working directory of the process, and tests must not write there. The guard is covered by the R3.3 argument-array test (-c protocol.file.allow=never for CloneTransport::Remote) and target-facts §11 SMOKE.
3. Cache directory in messages is the configured location (location.path()), not the real path; an entry's directory is location/<entry name>.
4. A work-tree source's clone argument is its .git folder (git's suffix order), as in the reference; a bare repo is its own path. Tests that delete the source use a bare copy for that reason.
5. Existing-entry-equal-to-clone is reported as ReadHow::Fetched; R3.10 can map it to Cached if the resolved stack needs the difference.
6. Account-folder and '..' refusals for a git: source are values (OriginRefusal::into_diagnostic); wiring them into the stack resolution is R3.10.
<!-- SECTION:NOTES:END -->
