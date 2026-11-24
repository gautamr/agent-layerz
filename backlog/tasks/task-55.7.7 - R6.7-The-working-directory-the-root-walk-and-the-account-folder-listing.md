---
id: TASK-55.7.7
title: 'R6.7: The working directory, the root walk and the account folder listing'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T10:25:00Z'
labels:
  - area:rust
  - area:accounts
  - area:security
milestone: m-7
dependencies:
  - TASK-55.7.6
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 166390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.13 "Paths", §9.1, decision-19, TASK-52).
- account/place.rs: WorkingDirectory::resolve(self, &AccountFolders) -> Place consumes the captured directory, so it is resolved exactly once; the R3.5 component walk checks each prefix before the next component; a directory that names an account folder as written is refused before any call. Place = InAccountFolder | InConfiguredFolder(real) | AtOrAboveHome | Directory(real).
- account/root_walk.rs: the nearest layer.yaml with a stack key; a layer's own manifest is skipped; one that cannot be used (link, folder, over 1 MiB, unreadable, invalid YAML, alias limit, schema) stops the walk with one warning naming the file; never a layer.yaml at or above HOME or in an account folder.
- account/listing.rs: one listing of $HOME: a folder or link named .claude-<name> or .codex-<name> exists; alz-blocked never counts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The working directory is resolved exactly once per run on macOS and Linux: an alz-io test counts the lookups through the walk's probe seam, and the resolver takes the Place by value (cargo test)
- [ ] #2 Property: neither walk lstats, reads a link of or opens a name inside an account folder, even through a link planted in the repository (proptest; TASK-52 #4, account side)
- [ ] #3 The root walk stops at each kind of unusable layer.yaml with one warning naming the file and never goes past it (integration tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (R6.7). alz-core path/walk: RealPathWalk::locate (Intent::Locate) for a caller that only wants the place: a folder that holds a configured folder is a real path, a name inside an account or configured folder ends the walk with Stop::Inside { relation, at } before any lstat there, home is Refused(Home); Stop lost Copy. alz-io: fs/walk.rs locate() -> Located; account/place.rs (Place = InAccountFolder | InConfiguredFolder(real configured folder) | AtOrAboveHome | Directory(real) | Unresolvable; WorkingDirectory::resolve(self, &AccountFolders) consumes the directory; resolve_with takes a PlaceLookup seam; as-written account folder or home makes no lookup; WorkingDirectory::capture_with injects the process lookup); account/root_walk.rs (find_root(place by value, folders, homes, &ReadOnlyDir on /) -> RootSearch { repository, known_directory, warnings }; ManifestState Read | Unreadable | Invalid; one account-manifest warning); account/listing.rs (AccountFolderNames::list: one listing of HOME, folder or link, alz-blocked never). R6.6 follow-up: Code gained account-home, account-file-ignored, account-xdg-relative, account-file-in-work-tree, account-config-{shape,key,default-name,folder,duplicate}, account-default-name-folder (code only; no producer yet), account-allowed-{shape,entry} and account-manifest; account-config and account-allowed are gone; invalid YAML in either file is account-file-ignored; texts unchanged. Evidence: AC1 crates/alz-io/tests/account_walk.rs the_working_directory_is_looked_up_once_and_resolved_once_for_the_whole_flow (counting PlaceLookup and capture_with closure both 1; resolve and find_root take the value by move) and a_directory_that_names_an_account_folder_or_home_as_written_makes_no_lookup (0 lookups). AC2 proptest nothing_inside_an_account_folder_is_looked_at_through_a_planted_link (account folders mode 000, link planted in the repo, result is AccountFolder with no warning; any lstat inside would give Unresolvable) plus listing test with mode-000 account folders and alz-core walk tests. AC3 each_unusable_layer_yaml_stops_the_walk_with_one_warning_naming_the_file (link, folder, over 1 MiB, unreadable, invalid YAML, alias limit, schema; the valid outer root is never reached). Also: nearest stack wins, layer's own manifest skipped, stops at HOME and at the file system root, configured-folder cases. Checks: mise run rust:ci exit 0; mise run lint ok. Open spec needs for R6.2: (1) a working directory in a configured folder is placed by the configured folder's real path only, so an allowed.yaml root strictly below that folder (and at or above the real cwd) is not matched; the spec says 'the working directory or one of its parents'. (2) warning shapes: the spec's 'accounts.yaml is ignored: ...', and the quoting of keys as "<key>" differ from the Rust where-prefix with line:column and 'key' quoting; decide which wins. (3) account-default-name-folder has no producer yet (R6.9). (4) CI log shows no account-xdg-relative/work-tree code assertion beyond the existing tests.
<!-- SECTION:NOTES:END -->
