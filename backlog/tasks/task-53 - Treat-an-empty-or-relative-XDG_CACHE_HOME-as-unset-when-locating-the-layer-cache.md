---
id: TASK-53
title: >-
  Treat an empty or relative XDG_CACHE_HOME as unset when locating the layer
  cache
status: To Do
assignee: []
created_date: '2026-11-22T08:16:00Z'
labels:
  - discovered
  - area:resolve
  - area:security
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
- [ ] #1 With XDG_CACHE_HOME set to an empty string, alz build writes the layer cache to ~/.cache/agent-layerz under the fake HOME and creates nothing in the repository (acceptance test comparing the repository tree before and after)
- [ ] #2 With XDG_CACHE_HOME set to a relative path, alz build ignores it the same way and uses ~/.cache/agent-layerz (acceptance test)
- [ ] #3 cacheRoot and account resolution compute the cache location with the same helper (unit or acceptance test showing both give the same path for unset, empty, relative and absolute values)
<!-- AC:END -->
