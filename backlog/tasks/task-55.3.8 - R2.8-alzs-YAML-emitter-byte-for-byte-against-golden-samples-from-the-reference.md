---
id: TASK-55.3.8
title: >-
  R2.8: alz's YAML emitter, byte for byte against golden samples from the
  reference
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
labels:
  - area:rust
  - area:render
milestone: m-7
dependencies:
  - TASK-55.3.4
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 97390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. decision-28 "Generated YAML": a small alz-owned emitter, tested byte for byte against the TypeScript output, whose samples become golden tests in R2. The spike's 229 samples lived in a temporary scratchpad and are gone (TASK-55.1.1 notes), so this step makes new ones.

- Generator: `crates/alz-core/tools/yaml-emit-samples.mjs` (Node, like `crates/alz/tools/untrusted-code-points.mjs`; it reads `src/` and node_modules and never changes them) calls the reference's `stringifyYaml` (`src/core/yaml.ts`: lineWidth 0, minContentWidth 0) on a corpus of what alz writes: agent, rule and skill frontmatter maps, nested native maps, lists, booleans, integers, empty collections, the allowed.yaml list with its comment, and the scalars the spike listed (yes, no, on, off, 1_000, 1.0.0, dates, 12:30, "true", "123", "0x10", ".inf", "~", "", multi-line text with zero, one and two final line feeds and with leading spaces, control characters, non-ASCII, U+2028, keys that need quoting). It writes `crates/alz-core/tests/fixtures/yaml-emit/<case>.json` (input) and `<case>.yaml` (expected), committed. Keys that are whole numbers are left out and listed for TASK-66 (R5).
- Emitter: `crates/alz-core/src/render/yaml.rs`; input an `EmitValue` (ordered map with text keys, lists, text, booleans, integers), output a `String`. Pure and total.
- The quoting rule it implements goes into the spec: written by the architect from the samples (comment on R2.1, or a spec follow-up task if R2.1 is closed).

Laws (proptest): `yaml::parse(emit(v))` gives back `v` through R2.4's parser; emit is deterministic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every committed sample is reproduced byte for byte (mise run rust:check), and the notes list the corpus by scalar class
- [ ] #2 The round-trip property test emit then parse passes
- [ ] #3 The quoting rule, with the samples that show it, is in the spec or handed to the architect in a comment
<!-- AC:END -->
