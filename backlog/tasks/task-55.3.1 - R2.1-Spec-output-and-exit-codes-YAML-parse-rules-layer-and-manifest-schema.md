---
id: TASK-55.3.1
title: 'R2.1: Spec: output and exit codes, YAML parse rules, layer and manifest schema'
status: Done
assignee:
  - '@architect'
created_date: '2026-11-23T05:17:00Z'
updated_date: '2026-11-23T06:55:00Z'
labels:
  - area:spec
  - area:rust
milestone: m-7
dependencies: []
parent_task_id: TASK-55.3
priority: high
type: docs
ordinal: 90390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. The architect writes the `docs/LAYERING-SPEC.md` text that R2's code follows, and the owner reviews it before R2.3 starts. User-facing: it changes messages and parse behaviour of the Rust binary. Nothing is released in R2; cutover (R8) carries the minor 0.x bump decision-28 asks for, and these changes go into its version note.

- New section "Output and exit codes" (decision-26: written in the pull request that builds the first Rust command exposing it, and R2.10's `alz lint` is that command): stdout for results and summaries (`lint ok: <name>`, `lint failed: <n> error(s) in <name>`), stderr for diagnostics and clap's usage errors; the line `<level> [<code>] <where>: <message>`; exit codes 0, 1, 2 and 70; the three text classes; the panic message. §10 points to it.
- Diagnostics about untrusted files: a schema error names the field by its key path (`'account.claude'`, `stack[1].ref`) and what is wrong, never the value (owner answer 4 in TASK-55.3's notes; reference drift where the reference prints more). A key taken from the input is quoted, stripped (class 1) and cut after 64 code points, counted after stripping, then `…` (owner answer 1). A YAML error gives the file, line, column and kind only.
- §7.13 "YAML limits": the control-character sets for raw text and for decoded strings, as decision-28 (amended 2026-11-23, owner answer 3) sets them: raw text may hold tab, line feed and carriage return, decoded strings only tab and line feed; explicit tags outside the YAML 1.2 core set (`yaml-tag`); more than one document; cyclic and unresolved aliases (TASK-60, TASK-59); invalid UTF-8; the code of each, and the reference drift from decision-28's Consequences list.
- §5.1: the rules the reference applies that the spec never wrote: the layer name pattern, `kind` as required text, every text field (`version`, `description`) must be YAML text, so a number is an error that suggests quoting (decision-28; `version: 1.10`), slots and `forbid_patterns` as lists of text.
- §5.2: stack entry fields (`source`, `ref`), which sources take `ref:`, the git ref rule, a ref that is not text (TASK-61), targets, workspace, budgets (whole numbers, `2.0` counts, decision-28), options. §5.3: how `layer.lock` is read: at most 1 MiB, strict UTF-8, nesting depth at most 128, a duplicate key makes it invalid (decision-33).
- §13: entries for R2.11's new acceptance file(s).

Out of scope: what R3 to R6 build (resolved sources, leak guard, merge, account resolution).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The spec has an 'Output and exit codes' section and §10 points to it; every diagnostic code R2 emits is named in the spec (grep of the codes in crates/ against the spec, recorded in the notes)
- [x] #2 §5.1, §5.2, §5.3 and §7.13 state every rule R2.4 to R2.9 enforce, each with its reference drift and task where the TypeScript alz differs (owner review comment on this task)
- [x] #3 The text follows the owner's answers in TASK-55.3's notes: input keys cut at 64 code points after stripping, CR allowed in raw YAML text only, schema errors never print values (owner review comment)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Architect (2026-11-23): spec text written in docs/LAYERING-SPEC.md, uncommitted on feat/rust-r2-task-55.3:
- §5.1 'Field rules' (name pattern, kind, visibility, text fields, numbers in text fields, stack field, layer.yaml missing or not a regular file);
- §5.2 'Field rules' (stack entries, ref rule and TASK-61, targets, workspace, budgets, options);
- §5.3 'Reading the lock' (1 MiB, UTF-8, depth 128, duplicate keys, lockVersion);
- §7.13 'What else makes a YAML file invalid' (control-character sets per decision-28 as amended, tags, documents, aliases, keys, types, error codes, drift list);
- §10 pointer and new §10.1 'Output and exit codes' (streams, diagnostic line, schema errors with key paths and the 64-code-point cut, exit codes, the three text classes, help and version, panic message, lint summary);
- §13 items 26 (34-yaml-limits) and 27 (35-manifest-schema).
mise run lint passes (biome, tsc; the spec itself is not linted). Messages quoted from the reference were checked in src/resolve/layer-load.ts.
Open:
- AC #1: the grep of the codes in crates/ against the spec can only run once R2's code exists, so it is left for R2.11. error [not-ported] is deliberately not in the spec: it is temporary (owner answer 5) and is never released.
- AC #2 and AC #3 wait for the owner's review comment.
- Kept as the reference has it, not decided by the owner: a ref: field on a github: or git: entry is checked but has no effect. Making it an error would be a new rule.

Owner approved the text (comment). AC #1 (grep of codes in crates/ against the spec) is checked in R2.11, once R2's code exists; R2.3 may start on the approved text.

R2.11 (2026-11-23): AC1 grep of the codes emitted in crates/ against docs/LAYERING-SPEC.md: all named except not-ported (deliberate) and account-config / account-allowed, which R2.11 added to 7.13. AC1 itself left for the lead to check.

AC #1 (2026-11-23, R2.11): codes emitted in crates/ are yaml, yaml-tag, layer, manifest, no-manifest, lock, source, account-config, account-allowed and the temporary not-ported; all but not-ported are named in the spec (account-config and account-allowed added to §7.13 in R2.11).
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: owner
created: 2026-11-23T05:45:00Z

---

Owner review (2026-11-23): the R2.1 spec text in commit 531859d is approved; the R2 unit review checks it against the code at the end.

---
<!-- COMMENTS:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The spec gained §10.1 Output and exit codes, the field rules and layer.yaml read errors of §5.1/§5.2, reading the lock in §5.3 and what else makes YAML invalid in §7.13, each with its reference drift; every code R2 emits is named in it.
<!-- SECTION:FINAL_SUMMARY:END -->
