---
id: TASK-131
title: Make the README a guide for people who use alz
status: In Progress
assignee:
  - '@claude'
created_date: '2026-11-28T02:59:00Z'
updated_date: '2026-11-28T15:51:00Z'
labels:
  - area:docs
milestone: m-1
dependencies:
  - TASK-7
priority: medium
type: docs
ordinal: 232390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The README mixes user and developer content and has four errors: alz:build is said to need the personal layer next to the clone (it comes from GitHub, cached); the examples pin @v1.0.0 while the personal layer's newest tag is v0.5.0; Status calls account switching planned although alz account which and allow exist; the 0.1.0 section covers only the registry install, not a from-source one. Developer content moves to CONTRIBUTING.md (TASK-7). Out of scope: release binaries and Homebrew, which do not exist yet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 README sections serve a user: what it is, status, why, install (Homebrew, mise, from source), uninstall, quick start, layers, commit identity, commands, safety, the algebra; the Development section is replaced by a link to CONTRIBUTING.md (read the file)
- [x] #2 Every command and option in the README exists in the CLI today (alz --help and each subcommand's --help)
- [x] #3 The quick start runs as written in a scratch repository
- [x] #4 The four errors in the description are fixed (read the file)
- [x] #5 No migration section: the README presents alz as a new release (owner, 2026-11-28)
- [x] #6 A status section near the top shows each coding agent's adapter status and each feature's status with ✅ 🔜 🙋
- [x] #7 Install lists Homebrew and mise first, marked as coming with the first binary release, then from source, which works today
- [x] #8 An algebra section states the merge structures and their laws as docs/algebra.md does, and links it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Checks: every command and option checked against alz --help and each subcommand's --help (doctor and lint rows reworded to match, the doctor row against a real alz doctor run). The quick start ran in a scratch repository with a real tagged public layer: init, build --adopt (54 files), ls and check --skip-identity passed. That run showed a layer's commit_identity becomes the scratch repository's git identity, so the README no longer names anyone's layer as one to try, and it says what a layer can set. alz init --kind layer scaffolded a layer.
Review (fresh-context reviewer) found and fixed: the 0.1.0 section could not tell the two installs apart because the Rust alz also reports 0.1.0 (it now tells them apart by symlink or regular file), and it said layers keep working when CHANGELOG says some fail; CHANGELOG cited the old heading; the cache path left out XDG_CACHE_HOME; the garbled settings bullet in Safety is now a list; 'the clone now fails' is now timeless.

Owner feedback, 2026-11-28: drop the 0.1.0 migration; a prominent status matrix with emoji, including adapters; the algebra and its laws; Homebrew and mise as the default install, then from source. Done. Homebrew and mise are marked 🔜 because no release, tap or release assets exist yet (TASK-55.8.1-55.8.4). The commands assume the tap repository gautamr/homebrew-tap (brew install gautamr/tap/alz) and mise's github backend (mise use -g github:gautamr/agent-layerz, which picks the release asset by OS and architecture). CHANGELOG no longer points to a migration section. Every relative link and anchor in README, CONTRIBUTING and docs/algebra.md resolves (script check).

Second review (on the algebra) fixed here: the Floor row names only the default_mode floor, with refusals in their own sentence; hook order is no longer listed under the union; rule chains are listed; deep merge is a semigroup on trees of one shape; 'every merge' is now 'most merges', and 'thousands' is gone; associativity is stated per structure, without the hub example; the coding-agent table says Subagents and lists Windsurf, like CONTRIBUTING; the emoji moved out of the Install headings so the anchors stay plain. Links and anchors in README, CONTRIBUTING and docs/algebra.md resolve (script check).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The README now serves people who use alz. A status section near the top shows each coding agent's adapter and each feature with ✅ 🔜 🙋. Install lists Homebrew and mise (🔜, from the first binary release), then from source (✅), plus uninstall. The quick start uses a placeholder layer and a note on what a layer can set. The doctor, lint and account text is corrected. A section on the algebra states the merge structures, their laws and the refusals, and links docs/algebra.md. The migration section is gone, at the owner's request, and the Development section is replaced by a link to CONTRIBUTING.md. Two fresh-context reviews; findings fixed. Depends on TASK-7.
<!-- SECTION:FINAL_SUMMARY:END -->
