---
id: TASK-62
title: parseYaml and parseYamlLimited apply the same alias limit
status: Done
assignee: []
created_date: '2026-11-22T16:27:00Z'
updated_date: '2026-11-23T07:29:00Z'
labels:
  - discovered
  - area:spec
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.3.11
references:
  - src/core/yaml.ts
priority: low
type: bug
ordinal: 68390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The yaml library's default maxAliasCount is 100, the same as MAX_ALIASES, so parseYamlLimited behaves exactly like parseYaml. The spec reads as if only some files get the alias limit; in fact all do. Neither caps input size: only some callers check 1 MiB first. Found by the YAML spike (TASK-55.1.1) against the frozen TypeScript reference. Decide per decision-21 whether TypeScript takes a fix (it must stay a correct oracle) or the spec records the intended Rust behaviour before R2 ports parsing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The spec states one parsing rule for all YAML alz reads (alias limit, size cap, depth) or names which files differ and why
- [x] #2 The Rust port's single YAML entry point follows that rule
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R2 (TASK-55.3, YAML with limits and the manifest model). Its acceptance cases assert the spec and are marked reference drift (test/drift.ts) where the TypeScript reference fails them. R3 depends on this task, so it closes within R2.

From TASK-65 (2026-11-23): spec §7.13 'The YAML alias limit' now defines the term as at most 100 aliases per document, counted over the whole document, plus decision-28's depth 64, nodes 20000 and replay 10000 budgets. The reference's yaml maxAliasCount: 100 counts uses per anchor, weighted by the aliases inside the anchored node (node_modules/yaml/dist/nodes/Alias.js), and refuses the 100th use of a plain anchor; SMOKE with yaml 2.9.1: 99 uses of one anchor parse, 100 are refused, 51 + 50 uses of two anchors parse. That counting difference is drift this task carries. Whether serde-saphyr max_aliases: 100 allows 100 or 99 aliases is unverified: R2's test pins the spec's 'at most 100'. Which files get the limits stays this task's.

TASK-65 review (2026-11-23): spec §7.13 'YAML limits' now states the one rule decision-28 (amended) gives. Every YAML file alz reads gets strict UTF-8, the 1 MiB cap and the four budgets: 100 aliases as written, 64 nested collections, 20000 nodes and 10000 replayed events. This covers layer.yaml, frontmatter (cap on the whole file), settings, accounts.yaml and allowed.yaml. So AC #1 is met by TASK-65's text, and what is left here is AC #2 (the Rust entry point) and the drift cases. The drift is listed in the spec. The reference caps only some files, reads U+FFFD and has no depth, node or replay budget. Its maxAliasCount counts per anchor: count starts at 1, weight from Alias.js getAliasCount, max over items, 1 for a scalar. So an anchor on an empty collection is never limited; SMOKE with yaml 2.9.1, 1000 uses of &a [] parse.

Architect (2026-11-23, R2 plan in TASK-55.3): fixed in the Rust model by R2.4 (TASK-55.3.4); closed by R2.11 (TASK-55.3.11), which adds the acceptance cases (referenceDrift where the reference fails) and checks these criteria with its run. This task now depends on TASK-55.3.11.

R2.11 evidence (2026-11-23): AC1 - spec 7.13 'YAML limits' states one rule for every YAML file alz reads (1 MiB, strict UTF-8, 100 aliases, depth 64, 20000 nodes, 10000 replayed events) and records the reference's differences. 34-yaml-limits proves each boundary against the Rust alz: 1 MiB lints and 1 MiB + 1 byte is error [yaml]; invalid UTF-8 refused; 100 aliases over three anchors lint and 101 are refused; 20000 nodes lint and 20001 are refused; 10000 replayed events parse and 10001 are refused. Each refusal is referenceDrift TASK-62 (the reference does not enforce it). AC2 - alz-core's clippy.toml bans every serde_saphyr entry point and granit_parser::Parser constructor (disallowed-methods); the only exceptions are the two #[expect] in yaml/events.rs and yaml/load.rs, so yaml::parse is the single entry point. mise run rust:ci exits 0.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed in the Rust alz in R2 (PR #7, decision-30); the TypeScript reference keeps the bug, and its acceptance cases are marked reference drift.
<!-- SECTION:FINAL_SUMMARY:END -->
