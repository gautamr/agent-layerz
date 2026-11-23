---
id: TASK-55.5.13
title: 'R4.13: Who may declare: the shared rule of commit_identity and account'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:28:00Z'
updated_date: '2026-11-23T15:13:00Z'
labels:
  - area:rust
  - area:merge
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.12
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 126390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.12, §7.13, decision-13).
- alz-core `merge/declarers.rs`: Declarer facts per layer (position in the stack, kind, visibility, Placement) and evaluate -> Declared<T> = None | Winner { layer, value } | Conflict, with the refused layers and their reasons (inside the repository; kind not client; public). The first layer, or one private kind: client layer, both outside the repository, may declare; the client's whole value wins; several clients, or a refused first layer, give Conflict. Build errors: one per refused layer and one naming every client when there are several.
- alz-io: Placement of each resolved layer (inside or outside the repository by its real path, a case variant of an in-repo path and a link into the repository counting as inside, as §7.12 says); reuse what R3 computes where it exists.
- The consumers (git config, alz account which and allow) are R6's.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The who-may-declare cases of tests 26 and 29 have Rust tests with the same verdict over in-memory stacks: repo layer, group layer, public client, second client, first layer inside the repository, confidential client, and no declaration
- [x] #2 Placement counts a path inside the repository, a case variant of one and a link into it as inside (alz-io integration test)
- [x] #3 Proptest: there is at most one winner, and a winner is the first layer or a private client layer outside the repository
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (R4.13), evidence. Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, all tests, doc, MSRV, deny, audit); mise run lint ok.
Files: alz-core merge/declarers.rs + declarers/tests.rs (Declared<T> = None | Winner{layer,value} | Conflict; Declarations<T> with declared(), refused() with Reasons, clients(); DeclarerState folded in stack.rs; Merged::commit_identity() and Merged::account() for R6), merge/stack.rs docs and Accumulator, merge/testing.rs (merged_placed), diagnostic.rs (Code::Identity 'identity', Code::Account 'account'), merge.rs exports. alz-io: resolve/placement.rs (placement_of_directory, placement_of_source, placements_of_stack), RepositoryPaths::as_written()/real(), tests/placement.rs.
AC1: 23 alz-core tests in merge::declarers::tests, same verdicts as tests 26/29. Mapping: 26 'rejects a declaration in the repo layer, whatever its kind says' -> a_declaration_in_the_repo_layer_is_refused_whatever_its_kind_says; 'group layer' -> a_declaration_in_a_group_layer_is_refused_and_names_its_kind; 'public layer that calls itself client' -> a_public_layer_that_calls_itself_client_is_refused; 'second client layer and names both' -> a_second_client_layer_is_refused_and_both_are_named; 'lists every client layer when more than two declare' -> every_client_layer_is_named_when_more_than_two_declare; 'first layer inside the repository' -> a_first_layer_inside_the_repository_is_refused_and_leaves_no_winner; 'client inside repo (symlink / git: path / file:// / x:y / case variant / link outside pointing in)' merge verdict -> a_client_layer_inside_the_repository_is_refused, with the placement itself in the alz-io tests (AC2); 'never prints control characters from a layer's kind' -> a_kind_with_control_characters_prints_without_them (decoded ESC is already a YAML error, so the test uses U+200B and U+202E); 'lets the client layer override the first layer and names it' / 'uses the first layer's identity when no client layer declares' / 'adds nothing when no layer declares' -> the_client_layer_overrides..., the_first_layers_identity_applies..., no_declaration_is_no_winner_and_no_error (winner is the verdict; the Commits text is R5/R6). 29 'takes the first layer's declaration' -> the_first_layers_account_applies...; 'client layer's whole value replaces the first layer's, a map included' -> the_client_layers_whole_account_replaces...; 'confidential client may declare' -> a_confidential_client_layer_may_declare; 'declares nothing when no layer does' -> no_declaration_is_no_winner_and_no_error; 'init prints a refused group declaration' -> a_group_layer_may_not_declare_an_account (error [account] group:layer.yaml; init's warning wording is R5/R6); also a_public_client..., two_client_layers_conflict_over_the_account..., the_two_fields_follow_the_rule_independently. Not covered here (need R6 or the CLI): test 26 identity cases of §9.4 and the git-source resolution cases that need git (source errors for '..' etc. are R3's).
AC2: alz-io tests/placement.rs (12 tests): inside path, outside path, sibling folder with the repo name as prefix, link outside pointing into the repo, link inside pointing out (inside as written), case variant (REPO/inner; folded by lies_within; the stronger Repo/INNER check runs only on a case-insensitive file system, which macOS is), unopenable dir counts as inside, path: source against repo root, github: outside, git remote outside unless a folder of that name exists in the repo, local git path by canonical path.
AC3: proptests a_winner_is_the_first_layer_or_a_private_client_layer_outside_the_repository (winner is a declarer, first or client, private+, Outside), there_is_a_conflict_exactly_when..., a_client_wins_over_the_first_layer_and_the_refused_layers_never_win.
Spec gaps (not edited): (1) §10.1 merge-diagnostics table lacks the codes identity and account, which §7.12/§7.13 errors use (reference emits [identity] and [account]); Code::Identity/Account added, spec needs two rows. (2) §7.13 says a declaration by a layer inside the repo, the first layer included, 'can neither win nor create a conflict' in resolution, while the shared rule (plan, reference evaluateDeclarations) gives Conflict for a refused first layer. Declarations::refused() exposes the refused layers with reasons so R6's resolver can apply the §7.13 resolution rule; spec should say which function yields which. (3) §7.12 says first layer must be outside the repo but does not say what a first-layer that is refused for placement means for a client that declares (reference and Rust: Conflict, no winner). (4) Placement of an unopenable/missing layer is Inside by choice (stricter; such a layer is not resolved anyway). (5) The wording of the refusal message is not in the spec; kept as the reference prints it, the acceptance tests only match fragments.
Open: R5/R6 must call placements_of_stack (or placement_of_source per layer) to build Placements; resolver does not yet return them.

Verified by the lead (2026-11-23): mise run rust:ci exits 0, lint passes, src/ and test/ unchanged. Lead: the build uses the shared rule (a refused layer is an error; a refused first layer gives Conflict); account resolution in R6 applies §7.13's rule (an in-repository declaration neither wins nor conflicts) through refused(); placements_of_stack is wired in R5/R6.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Who may declare commit_identity and account is one pure rule in alz-core: the first layer or one private or confidential client layer, both outside the repository, may declare; the client's whole value wins; several clients or a refused first layer give Conflict, with refusal reasons exposed for R6. alz-io places each layer inside or outside the repository by its real path, links and case variants. Tests 26 and 29's who-may-declare cases have Rust tests with the same verdicts.
<!-- SECTION:FINAL_SUMMARY:END -->
