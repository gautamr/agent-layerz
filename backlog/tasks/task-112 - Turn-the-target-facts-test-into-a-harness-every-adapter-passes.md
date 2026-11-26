---
id: TASK-112
title: Turn the target-facts test into a harness every adapter passes
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
updated_date: '2026-11-26T08:41:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-111
references:
  - decision-40
  - docs/target-facts.md
priority: medium
ordinal: 213390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/alz-core/src/target/claude/tests/facts.rs enforces spec §8 'rely only on recorded facts' for Claude with Claude-specific extractors. decision-40 point 7 generalises it: structured output lets one harness list the written keys of any adapter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The harness lists keys from structured output and checks them against an adapter's facts file and subject map: a fake adapter with an unrecorded key fails, a recorded key passes, a stale NO_FACT_RECORDS entry fails (tests)
- [ ] #2 The Claude facts test uses the harness, with its subject map and NO_FACT_RECORDS unchanged in content (diff)
- [ ] #3 Each descriptor names its facts file, and spec §8 says the rule applies to every adapter (grep)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (commit 'refactor(alz-core): one facts harness for every adapter (task-112)').
Harness: crates/alz-test-support/src/facts (test support, because it reads docs/target-facts.md; the pure crates keep no effects and an adapter crate takes it as a dev-dependency; alz-test-support gained the alz-adapter dependency, alz-core a dev-dependency on it). Key path form: keys joined by '.', '[]' for the items of an array, e.g. hooks.PreToolUse[].hooks[].type; Markdown frontmatter keys are paths from the frontmatter top; kept (SKILL.md) frontmatter lists top-level keys only. An adapter supplies a SubjectMap (kind + facts sections for a file location and for a key path; None = not alz's key) and an exemption list; check()/assert_rests_on_facts() return Findings: unplaced files, unrecorded keys, exemptions no output uses, exemptions the facts record now.
AC #1 evidence: crates/alz-test-support/src/facts/tests.rs with a fake adapter: a_key_that_no_verified_row_records_fails, a_key_that_a_verified_row_records_passes, a_stale_exemption_fails_when_no_output_uses_it_any_more, a_stale_exemption_fails_when_the_facts_record_the_key_now, plus unverified-row, location, unplaced-file, key-listing and facts-file-read tests. Also checked by hand: a bogus and a now-recorded entry in Claude's NO_FACT_RECORDS fail the Claude test.
AC #2 evidence: git diff of crates/alz-core/src/adapter_model/claude_stacks/facts.rs shows only structural change: Kind, Place and every subject() arm (sections, row_holds) are unchanged; NO_FACT_RECORDS is still the empty list; the file extractors, row lookup and Subject struct moved to the harness; the test now reads the facts file the descriptor names and gives the adapter's structured Output to assert_rests_on_facts. CLAUDE.md is filtered out as before (no frontmatter, no key).
AC #3 evidence: Claude's descriptor names docs/target-facts.md (crates/alz-target-claude/src/descriptor.rs FACTS_FILE) and the harness reads that path, refusing an absolute or '..' path. Spec section 8 What an adapter may rely on already reads 'An adapter relies only on what docs/target-facts.md records about the tool, ...' and section 8.0 Descriptor lists 'its facts file ... (section 8 What an adapter may rely on)' with TASK-112; no spec change made. docs/spec-coverage.md rows for section 7.9 Facts, section 8 What an adapter may rely on and section 8.0 Descriptor updated.
Checks: mise run rust:check, mise run rust:ci and mise run alz:check pass (67 generated files match).
<!-- SECTION:NOTES:END -->
