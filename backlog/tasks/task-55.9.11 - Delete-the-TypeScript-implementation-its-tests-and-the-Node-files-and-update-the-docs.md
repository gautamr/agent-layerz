---
id: TASK-55.9.11
title: >-
  Delete the TypeScript implementation, its tests and the Node files, and update
  the docs
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-25T13:45:00Z'
labels:
  - area:rust
  - area:docs
milestone: m-7
dependencies:
  - TASK-55.9.10
parent_task_id: TASK-55.9
priority: medium
ordinal: 193390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The last step of decision-39: with no gap in the coverage matrix and nothing running Node, delete src/, test/ (its fixtures already moved), package.json, package-lock.json, tsconfig.json, biome.json, vitest.config.ts, scripts/install-local.sh and docs/standards/typescript.md, drop the dist/ and node_modules/ lines from .gitignore, and delete each fixtures/ folder that no Rust test reads. README.md, CHANGELOG.md, FRAMEWORK-GAPS.md and evals/ then describe only the Rust alz: building from source and the local install (R7 adds Homebrew and releases), how to leave an npm install of 0.1.0 behind, and no npm command. Dropping the npm install path is user-facing and ships in the first Rust release (R7, TASK-55.8). Out of scope: distribution, and removing the npm package (owner).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 git ls-files src test package.json package-lock.json tsconfig.json biome.json vitest.config.ts scripts/install-local.sh docs/standards/typescript.md and git ls-files '*.ts' '*.mts' '*.cts' '*.js' '*.mjs' '*.cjs' print nothing (grep)
- [x] #2 Every folder under fixtures/ is read by at least one Rust test (the check's command and output in notes)
- [x] #3 git grep -niE 'npm|npx|node[.]?js|typescript|vitest' -- README.md CHANGELOG.md FRAMEWORK-GAPS.md evals docs .gitignore prints only CHANGELOG lines about 0.1.0 and lines the notes list (grep)
- [x] #4 Every command in README's install and development sections works in this checkout (each run once, output in notes)
- [x] #5 mise run rust:ci and mise run alz:check pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer, commit on branch chore/delete-ts-work (3d01fa8, on top of 93479a9). Deleted 189 tracked files: src/ 103 .ts; test/ 79 (78 .ts, 1 .json: test/ci/case-floor.json); package.json, package-lock.json, tsconfig.json, biome.json, vitest.config.ts, scripts/install-local.sh, docs/standards/typescript.md. Edited: .gitignore (node_modules/ and dist/ lines dropped), README.md, CHANGELOG.md, mise.toml. No FRAMEWORK-GAPS.md exists. Kept: evals/ (run.sh uses the Rust alz build --adopt, check.sh is a shell check of the fixture repo; neither used Node) and the npm deny rules in .layer/permissions.yaml (deny rules only get stricter; the npm package still exists). scripts/ is gone entirely; mise task install:local now holds the install steps itself (same behaviour as the script: no symlink, PREFIX/BIN_DIR/ALZ_NAME, SHA-256 check). .layer/ and the generated files mention no deleted path, so no rebuild changed anything (alz check passes on a scratch copy; layer.lock unchanged).
AC1 (met): git ls-files <the listed paths> | wc -l -> 0; git ls-files '*.ts' '*.mts' '*.cts' '*.js' '*.mjs' '*.cjs' | wc -l -> 0 (also no package*.json).
AC2 (met, reading; see the flag): all 32 folders under fixtures/ are read by Rust tests. crates/alz-io/tests/layer_contents.rs walks and hashes every layer folder below ../../fixtures, so none is unread. 16 folders are named by no Rust file (found with: for d in fixtures/*/; do n=$(basename $d); grep -rlF "$n" crates --include='*.rs' >/dev/null || echo $n; done): 10-leak-guard, 11-visibility, 14-non-destructive, 20-adopt-native, 20-adopt-native-plain, 28-cache-integrity, 29-account-layer, 30-account-cache-read, 31-account-which, 32-account-allow, 33-name-order, 34-yaml-limits, 35-manifest-schema, 37-settings-targets, 38-emitter-drift, 39-writer-rules. I kept them: they are inputs for the gap tests of 55.9.6 to 55.9.9. Owner/lead: if only a test that names the folder counts, delete those 16 (git history keeps them).
AC3 (met): git grep -niE 'npm|npx|node[.]?js|typescript|vitest' -- README.md CHANGELOG.md FRAMEWORK-GAPS.md evals docs .gitignore prints 3 lines, none in README, evals or .gitignore, none in CHANGELOG: docs/LAYERING-SPEC.md:1341 (an MCP server example, command: npx, user config not alz), docs/target-facts.md:70 (a recorded fact naming the skill gr-typescript-domain) and :409 (git and npm plugin marketplaces, a Claude Code fact). The CHANGELOG entry about leaving 0.1.0 avoids the words and says nothing about its security.
AC4 (met): README Install and new Development sections. Run in this checkout with PREFIX in the scratch folder: mise run install:local (first install, then update: prints 'previous alz 0.1.0'; sha256 73830a8d...), installed alz --version -> alz 0.1.0; BIN_DIR with a symlink 'alz' -> refused with the remove hint, exit 1; mise run rust:build ok; mise run rust:check ok (47 test binaries ok, 0 FAILED); mise run rust:ci ok (in AC5); alz:check ok (AC5). 'git clone' is not run (network); alz:build is the same build step as alz:check, needs the personal layer at ../agent-layerz-personal, run only in the scratch copy.
AC5 (met): mise run rust:ci -> exit 0 (fmt, clippy -D warnings, tests, doc, msrv, cargo-deny 'advisories ok, bans ok, licenses ok, sources ok', cargo-audit 84 crates). alz:check in a scratch copy of the commit (git archive) next to a read-only copy of the personal layer: 'check passed: 67 generated files match the layers and the lock', exit 0 (only the two known agent-replace warnings, and a note that the scratch copy has no .git). mise run workflows:lint not run for this change (no workflow touched); see the report. Open: .layer/agents/release-engineer.md still says it owns 'the tasks and scripts/ of the build' while scripts/ no longer exists (R7 may add scripts); left unchanged.

Update: mise run workflows:lint equivalent run with the shellcheck 0.11.0 path (the shim has no version): actionlint exit 0, zizmor --pedantic --offline: no findings.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
All TypeScript is deleted: src/, test/, the Node package and config files (189 files); the README installs through mise run install:local; fixtures/ kept, every folder hashed by layer_contents and used by the gap tests. Owner approved; merged in #26 (f85e367).
<!-- SECTION:FINAL_SUMMARY:END -->
