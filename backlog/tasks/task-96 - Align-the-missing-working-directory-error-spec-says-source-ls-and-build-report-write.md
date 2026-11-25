---
id: TASK-96
title: >-
  Align the missing working directory error: spec says source, ls and build
  report write
status: To Do
assignee: []
created_date: '2026-11-25T14:37:00Z'
labels:
  - bug
dependencies: []
ordinal: 199390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Spec §6 'Other messages of resolution' says a process with no working directory gets the error source, cannot find the working directory. Through a command that is not what happens: lint reads the relative layer.yaml first and reports a missing layer.yaml in layer '.', and ls and build report the same text as the error write, because open_repository (crates/alz/src/command/pipeline.rs) uses Code::Write. Only StackResolver::in_working_directory gives source, and no command reaches it. Decide whether the spec or the code changes, then align them and add a command-level test (found by TASK-55.9.6, test a_resolver_without_a_working_directory_is_the_error_source_cannot_find_it).
<!-- SECTION:DESCRIPTION:END -->
