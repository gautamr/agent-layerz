---
id: TASK-55.6.10
title: 'R5.10: alz ls and alz explain'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T04:55:00Z'
labels:
  - area:rust
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.6.8
  - TASK-55.6.17
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 142390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10), from Merged's provenance and the Rendered tree.
- listing.rs: the sections agents, skills, commands, rules, hooks and mcp (enabled servers), each with its count and, per item, the name padded as the reference pads it and the layer that decides it.
- explain.rs: types agent, skill, command, rule, hook, mcp and instruction; the sources general to specific with their actions; a defined but not enabled server says so; then the merged output file or files from the tree. An unknown type is a usage error (exit 2); an unknown name exits 1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 alz ls on fixtures 01 and 22 prints the sections, counts and deciding layers the acceptance tests assert (vitest run of test 22 without --live)
- [x] #2 alz explain for each type lists the sources in stack order and the merged file, and the explain cases of tests 03 and 17 pass against ALZ_BIN (vitest run)
- [x] #3 Untrusted names and actions are printed only as validated names or stripped text (unit tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (R5.10). Pure core in alz-core: listing.rs (Listing, Section, Decider, ListedItem; names stripped, padded to 28; stubs listed with skills, layer 'slot stub') and explain.rs (ItemType, explain(), Explanation, NoSuchItem; sources through SourceRef (header form) with a stripped fallback for an origin a header cannot hold; actions through ActionWord, stripped; '(overridden)' derived from a later Action::Replace for rule/skill/command/mcp, never for agents or hooks; stub = 'slot stub — stub'; instruction blocks = 'block'). Code::Explain and Code::Ls added. Binary: command/ls.rs, command/explain.rs, cli.rs (Ls, Explain with ItemTypeArg ValueEnum so an unknown type is clap's usage error, exit 2), pipeline::Computed gained the merged model. The merged output file text passes strip_untrusted_keeping_line_feeds (tabs and CR are therefore removed, as class 1 removes them). Seam for R5.11: ls takes no --live yet (clap usage error); command/ls.rs documents that --live reuses Computed after the listing is printed; tests 22 --live cases fail until R5.11.
Reference parity: ran the TypeScript ls/explain on every fixture repo with a stack (26 same, 13 diff) and explain for rule/agent/skill/command/hook/mcp/instruction/stub on fixtures 01,02,03,05,06,08,09,17,19: stdout identical everywhere; the only differences are drift (reference lists skills in layer order, spec says name order; unknown name has no 'error [explain]' prefix; both recorded in spec 10.3 and as referenceDrift cases) and diagnostic wording of other tasks (tool-scope, slot-missing, tool-widen, permission-floor, hook-locked, yaml position).

Verified by the lead (2026-11-24): merged into the R5 branch (52fde9b); mise run rust:ci exits 0. Test 22's two --live cases are R5.11's.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz ls lists the six sections with the layer that decides each item, and alz explain prints each item's sources and action words from the merged model's provenance, (overridden) where a later layer replaced it, stub and block where the reference prints them; untrusted text is stripped. Output equals the reference's on every fixture except two drifts the spec records.
<!-- SECTION:FINAL_SUMMARY:END -->
