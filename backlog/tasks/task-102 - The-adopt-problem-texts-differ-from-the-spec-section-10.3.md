---
id: TASK-102
title: The adopt problem texts differ from the spec (section 10.3)
status: Done
assignee: []
created_date: '2026-11-25T17:22:00Z'
updated_date: '2026-11-25T18:12:00Z'
labels:
  - bug
dependencies: []
ordinal: 204390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Four problem lines of 'alz build --adopt' do not have the words section 10.3 gives. (1) A server env entry: the spec says 'server <id> sets <NAME> to a value', the build quotes the name ('TOKEN'). (2) A server with unsupported fields: the spec says 'uses <keys>, which mcp.yaml does not support yet', the build says 'uses fields that mcp.yaml does not support yet' with no key names. (3) A permission rule with an unknown tool: the spec says '<file>.<list>: <rule> uses a tool agent-layerz does not know', the build says '<file>: permissions.<list>: <rule> uses a tool agent-layerz does not know'. (4) Text the layer reader refuses: the spec says '<file>: adopting it would write <layer file> with text the layer reader refuses: <rule>; fix it in <file> and rerun', the build says '<file>: the repo layer that adoption plans would be refused: <full reader line>'. Decide which side is right, change the other, and add the missing texts to crates/alz/tests/adopt_output.rs. Found while closing the section 10 gaps of TASK-55.9.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in Rust (decision-30, the spec is the contract): the four plan problems of 'alz build --adopt' now have the words of section 10.3. Server env entry: server 'db' sets TOKEN to a value (the name is no longer quoted). Unsupported fields: server 'db' uses timeout, cwd, which mcp.yaml does not support yet (the keys are listed, stripped, in file order). Unknown tool: '<file>.<list>: '<rule>' uses a tool agent-layerz does not know' (file and list joined by a dot). Reader refusal: '<file>: adopting it would write <layer file> with text the layer reader refuses: <reader message>; fix it in <file> and rerun' (crates/alz-core/src/adopt/check.rs). Test: crates/alz/tests/adopt_output.rs::the_server_permission_and_reader_problems_have_the_words_of_the_spec asserts each exact line and that no value is printed; unit tests in crates/alz-core/src/adopt/tests.rs updated. Matrix row section 10.3 alz build is covered; the several-targets clause is listed under Known differences as TASK-18. rust:ci passes.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The four build --adopt plan-problem messages read as the spec words them; adopt_output.rs asserts each line. Merged in #34 (e027d1f).
<!-- SECTION:FINAL_SUMMARY:END -->
