---
id: TASK-35
title: Install alz from source as one self-contained file
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-21T06:43:00Z'
updated_date: '2026-11-21T06:45:00Z'
labels:
  - area:repo
milestone: m-1
dependencies: []
priority: medium
ordinal: 1390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Developers who clone the repository need a simple local build-and-install that leaves no symlink and no node_modules at the destination. The existing mise task install:local installs through npm pack, and npm always creates ~/.local/bin/alz as a symlink into lib/node_modules; the owner does not want links. The install must work for anyone who clones the public repository, with or without mise, on macOS and Linux (Windows support is undecided, TASK-23).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A POSIX script builds alz with its runtime dependencies bundled into one file and installs it as a regular executable (not a symlink) at ${BIN_DIR:-${PREFIX:-$HOME/.local}/bin}/alz
- [x] #2 The script checks Node.js 22.12 or newer, refuses to replace a symlink at the target and says how to remove it, replaces the file atomically, and prints the installed path, version, git revision and SHA-256, failing if the installed hash differs from the built one
- [x] #3 `mise run install:local` runs the script; the script also runs without mise
- [x] #4 A test proves the standalone bundle runs from a directory with no node_modules
- [x] #5 README documents installing from source; the repo command table lists the task
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. package.json: build:standalone script (esbuild bundle with yaml and diff included, createRequire banner) -> dist/alz-standalone.mjs.
2. scripts/install-local.sh (POSIX sh): Node >= 22.12 check, refuse a symlink target, npm ci if node_modules missing, build:standalone, atomic copy with mode 755 to BIN_DIR or PREFIX/bin (default ~/.local/bin), compare SHA-256 (sha256sum or shasum), run --help, print path, version, git revision, hash, PATH note.
3. mise.toml: install:local runs the script (replaces the npm pack version).
4. Test: build the bundle, copy it to an empty temp dir, run --version and --help there.
5. README "From source" install; .layer/instructions/50-repo.md command table; mise run alz:build.
6. mise run test, lint, alz:check; run the script with BIN_DIR in scratch.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified 2026-11-21: test 27-install-local (2 tests) passes: regular file, mode 755, not a symlink, hash equals the build, runs --version and --help from an empty directory; refuses a symlink target and leaves it. Script run directly and through mise run install:local with BIN_DIR in a scratch dir (identical SHA-256 on reinstall). A fake node v22.11.0 on PATH makes the script fail with the version message and install nothing. Full suite 26 files, 135 passed, 1 skipped; lint clean; alz:check passed. mise.toml install:local replaced (was npm pack + global install, which symlinks the bin).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Adds a source install that leaves no symlink: scripts/install-local.sh (POSIX sh) builds dist/alz-standalone.mjs with npm run build:standalone (esbuild bundle including yaml and diff, createRequire banner) and installs it atomically as a regular 755 file at ${BIN_DIR:-${PREFIX:-$HOME/.local}/bin}/alz. It checks Node >= 22.12, refuses to replace a symlink (e.g. from npm install --global), compares SHA-256 of build and install, runs the installed binary, and prints path, version, git revision and hash. mise run install:local now calls it (replacing the npm pack variant); README documents installing from source; the repo command table lists the task.

Verification: 27-install-local acceptance test (2 tests); manual runs direct and via mise; Node version check proven with a fake old node; full suite 135 passed, 1 skipped; lint and alz:check clean.

Notes: macOS and Linux only (Windows undecided, TASK-23). The standalone bundle includes the dependencies, so the test suite (which runs dist/cli.js) does not cover every command through it; the new test covers --version and --help, and alz check and init were run with it manually.
<!-- SECTION:FINAL_SUMMARY:END -->
