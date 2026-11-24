---
id: TASK-55.6.1
title: 'R5.1: Spec for the writer, the Claude output, adoption and the R5 commands'
status: Done
assignee:
  - '@architect'
created_date: '2026-11-23T18:08:00Z'
updated_date: '2026-11-23T22:54:00Z'
labels:
  - area:rust
  - area:spec
milestone: m-7
dependencies: []
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 133390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step: the spec text R5 builds against, written once the owner has answered the R5 questions in TASK-55.6's notes. No code.
- §4.3: the writer. A file is written or deleted only when alz can show it generated it (owner question 1); never through a link (the entry or any parent folder); paths compared with ASCII case folding of the NFC form; the lock written last; the managed .gitignore block (current and legacy markers); the manifest edit of --adopt (owner question 2).
- §4.5: protected paths match with ASCII case folding.
- §7.1 and §4.3: the generated header, its source refs, and the rule that a ref holds no control or format character and cannot close its comment (owner question 6).
- §8.1: the Claude output per item, including the three permission floor keys once TASK-76 records them, and what is written for each neutral tool and mode.
- §10 and §10.1: each R5 command's output lines and exit codes, the diff format (owner question 3), and the codes collision, check, budget, protected, adopt, target, claude-agent, tool-scope, hook-on-fail and ls.
- §13: entries for the writer rules and the parity goldens. CHANGELOG entry (user-facing, minor bump at cutover).
Out of scope: Codex output (TASK-18), §9.4 identity behaviour (R6).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/LAYERING-SPEC.md states every rule listed in this task's description, each owner answer of TASK-55.6 included (spec diff, reviewed)
- [x] #2 §10.1 lists each new diagnostic code with its level and what raises it, and §10 quotes every R5 message the acceptance suite asserts (spec diff checked against grep of test/acceptance)
- [x] #3 CHANGELOG.md names the user-facing changes and the minor bump at cutover; mise run lint exits 0
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Architect (2026-11-23, R5.1 spec pass, uncommitted on feat/rust-r5-task-55.6; it lands with the R5 pull request). I followed the owner answers and lead decisions in TASK-55.6's notes.

Spec changes (docs/LAYERING-SPEC.md):
- §4.3, after the adoption table:
  - the header and its sources (`<layer>@<rev>:<path>`, the YAML comment form, the generated mark and its earlier spelling; a source holding `-->` is error layer);
  - "Which files are alz's own": the generated mark, or a lock entry whose current bytes match the recorded hash; nothing else is ever overwritten or deleted; one bullet per case of the build comparison;
  - links refused: any path under a link, and any entry that is not a regular file, is a collision;
  - paths that differ only in case: NFC, then ASCII lower case;
  - writing: temporary file and rename, modes, the lock last;
  - the .gitignore block;
  - "Adoption, in detail": the repo layer requirement, the planned layer read with every layer check (TASK-73), problems stripped, JSON floats exact, the YAML merge, and alz's own minimal text edit of layer.yaml that keeps comments and every other byte, refusing the forms it does not handle.
- §4.5: the glob syntax, case-insensitive matching, the protected warning.
- §5.3 "File names": names with a control or format character are refused (error layer, layer-invalid in resolution), with the order of checks updated.
- §7.1: the budget messages, Commits section included.
- §7.11: the case-folded passthrough collision.
- §8 intro: "What an adapter may rely on". Emit exactly what the reference emits, nothing new on an UNVERIFIED fact, and a docs task per UNVERIFIED row; two such rows are named. The floor keys wait for TASK-76. codex is error target.
- §8.1: the Claude output per item, with the tool and mode mappings, hooks, settings.json, .mcp.json, protected and managed folders, the adopt-always files and the verified range.
- §10.1: the `note` level, and a table of the R5 codes: collision, check, budget, protected, layer, passthrough, target, claude-agent, tool-scope, hook-on-fail, adopt, explain, ls, claude, init.
- New §10.3: the output of build, check, diff (alz's own Myers unified format), ls and --live, explain (with its action words), doctor and init.
- §13: entries 30 (the writer and generated headers) and 31 (output parity with golden trees).
Other files: CHANGELOG.md has an Unreleased entry for the writer and adoption changes, a minor bump at cutover. backlog/decisions/decision-33 is amended ("Amended 2026-11-23 (owner, R5, TASK-55.6)", float_roundtrip on; title and status unchanged; the deny.toml change is listed under consequences for the implementing PR).

Checks:
- `mise run lint`: exit 0, run twice, the second time after the last edit.
- Leak grep over `git diff docs/ CHANGELOG.md backlog/decisions/`: no private-task term. The one hit for "private" is the word in the init options.
- Both private task files are still git-ignored.
- AC #2: a script checked 26 message strings that the acceptance suite asserts for R5 commands (ls, --live, doctor, init, adopt, check, budget, collision, explain) against the spec text; all 26 are present.
- AC #1 is not ticked: the spec diff still needs its review.

Findings for the coordinator (not done here; they need a subtask or a plan note):
1. Provenance gap. alz-core's Contribution (merge/provenance.rs) holds only the layer and an Action (Define, Extend, Replace, Remove). Headers need the file of each contribution and the layer's rev or version: test 26 asserts `+ client@0.1.0:layer.yaml`, and the reference's header sources are file refs. explain needs the kind's action words that §10.3 now states: test 17 asserts `2. shop-api:rules/testing.md — extend: paths +[e2e/**]`. R5.3, R5.4 and R5.10 cannot be byte-identical without that. Proposed: a small alz-core step before R5.3 that gives Contribution the file (SafeRelPath) and an action detail per kind.
2. The §5.3 refusal of control and format characters in layer file names changes the alz-io walk and the scan verdict (R3 code). No R5 subtask names it yet. It fits R5.8 (pipeline) or a small step of its own.
3. The owner asked for a docs task for each UNVERIFIED row the Claude output relies on: relative additionalDirectories (target-facts §7c) and an unknown settings key (target-facts §7, which TASK-79 may make moot). I did not create it, since this session touches only docs/, decisions and R5.1.
4. R5.1 still carries the label needs:owner-approval in its file, although the answers are in.

Architect (2026-11-24, R5.1 additions asked by the coordinator after R5.2 was committed):
- §4.3 "Paths that differ only in case" and §4.5 fold paths with the NFD form, then ASCII lower case, replacing the NFC wording of my first pass. So É and é collide, and letters without an ASCII base stay apart. Another writer had already put NFD into §4.3 while I was editing; I only rewrapped that paragraph and added the same rule to §4.5.
- §4.5: ? matches one Unicode scalar value; the reference counts a UTF-16 unit (drift). The parenthesis was already there from the other writer; I rewrapped it.
- §4.3 header: a source holds no control or format character and no -->. One that does, through a layer version, a ref or a path, is error layer, with the message updated. The §10.1 layer row and §13 entry 30 now say the same.
- §10.1 already lists the codes collision and protected (my first pass).
- CHANGELOG: the accent folding, the wider header rule, and ? above U+FFFF.
Checks: mise run lint exit 0; the leak grep over the uncommitted docs, CHANGELOG and decision diff found 0 private-task terms.

Owner approved the R5 spec (2026-11-24) as committed in 4592b6f.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The R5 spec is in docs/LAYERING-SPEC.md: header and ownership rules (the mark, or the lock plus a matching hash), links and non-regular files as collisions, NFD and ASCII case folding, refused file names and header sources, the glob syntax, the Claude output per item, the R5 codes and the output of build, check, diff, ls, explain, doctor and init; decision-33 amended for float_roundtrip. Owner approved.
<!-- SECTION:FINAL_SUMMARY:END -->
