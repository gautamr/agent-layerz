---
id: TASK-53
title: >-
  Treat an empty or relative XDG_CACHE_HOME as unset when locating the layer
  cache
status: In Progress
assignee: []
created_date: '2026-11-22T08:16:00Z'
updated_date: '2026-11-24T03:36:00Z'
labels:
  - discovered
  - area:resolve
  - area:security
  - area:rust
milestone: m-3
dependencies:
  - TASK-50.10
references:
  - docs/LAYERING-SPEC.md
priority: low
type: bug
ordinal: 30390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while planning TASK-50.10. cacheRoot (src/resolve/cache-paths.ts) uses XDG_CACHE_HOME whenever it is defined, so an empty value gives the relative path 'agent-layerz' and a relative value gives '<value>/agent-layerz'. build then creates the cache relative to the process working directory, usually inside the repository being built, outside insideRoot and outside ~/.cache. The XDG Base Directory specification says an empty or relative value is to be ignored. TASK-50.10 adds a shared XDG location helper for account resolution that already follows that rule; this task makes build and every command that resolves the stack use it, so build and resolution agree on the cache location. Out of scope: account resolution itself (TASK-50.10).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 With XDG_CACHE_HOME set to an empty string, alz lint in a repository with a git: layer writes the layer cache to ~/.cache/agent-layerz under the fake HOME and creates nothing in the repository (acceptance lint twin comparing the repository tree before and after, R3.11)
- [x] #2 With XDG_CACHE_HOME set to a relative path, alz lint ignores it the same way and uses ~/.cache/agent-layerz (acceptance lint twin, R3.11)
- [x] #3 alz lint and account resolution compute the cache location with the same helper, CacheLocation (R3.5) (unit test showing it gives the same path for unset, empty, relative and absolute values)
- [ ] #4 The alz build forms of AC #1 and AC #2 is in the acceptance suite (written in R3.11 where it is missing) and is checked again against the Rust alz in R5 (TASK-55.6)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22: paused by decision-21 (Rust port, doc-5). Built in Rust in TASK-55.4, where openat-relative access closes it properly.

Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R3 (TASK-55.4, sources and the cache). Its acceptance cases assert the spec and are marked reference drift (test/drift.ts) where the TypeScript reference fails them. R4 depends on this task, so it closes within R3.

Architect (2026-11-23, owner answer 7 in TASK-55.4, delegated by the lead): criteria reworded to the lint twins that R3.11 adds; the build, check and account-resolution forms are checked again in R5 (TASK-55.6) or R6 (TASK-55.7), as the last criterion says. The other criteria are unchanged.

R3.11 (2026-11-23): AC1-3 checked. Lint twins (empty and relative XDG_CACHE_HOME, repository snapshot before and after, cache in ~/.cache/agent-layerz under the fake HOME) pass against the Rust binary, marked referenceDrift TASK-53 (expected failure on the reference); CacheLocation unit tests (R3.5). AC4 left unchecked: build forms are in the suite and fail against Rust until R5 (TASK-55.6).

R5.8 (TASK-55.6.8, 2026-11-24): build form run against ALZ_BIN=target/release/alz in test 36 (16 passed, 4 Linux-only skipped) and test 24 (259 of 277; none of the 18 failures is this task's rows: --adopt R5.14, account which R6, agent mcpServers and nested JSON R4 follow-up). AC #4 left for the lead to check.
<!-- SECTION:NOTES:END -->
