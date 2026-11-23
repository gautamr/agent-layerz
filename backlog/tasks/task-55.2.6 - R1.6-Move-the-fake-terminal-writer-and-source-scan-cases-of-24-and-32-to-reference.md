---
id: TASK-55.2.6
title: >-
  R1.6: Move the fake-terminal, writer and source-scan cases of 24 and 32 to
  reference
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-22T17:56:00Z'
updated_date: '2026-11-23T02:32:00Z'
labels:
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.2.5
references:
  - test/allow-driver.ts
parent_task_id: TASK-55.2
priority: high
type: task
ordinal: 76390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`alz account allow` runs only for a person at a terminal, so test 32 and two cases of 24 call `run` in process with a fake `Terminal` that records prompts and gives scripted answers. A spawned binary needs a real pseudo-terminal for these cases, which is R1.8 and waits on an owner choice. Until then they move to reference, so the TypeScript code stays fully covered. Same rule as R1.3.

- To test/reference/32-account-allow.test.ts (and 24's two cases to test/reference/24-untrusted-repo.test.ts):
  - every case that passes a `Terminal` or uses `createTerminal`;
  - the 12-process `updateAllowed` lock case with test/allow-driver.ts (it moves under test/reference/);
  - the import-graph and "only terminal.ts reads standard input" scans;
  - the trace cases.
- Stays in acceptance, driven through the CLI with no terminal:
  - refusal without a terminal: a pipe, no stdin, each agent-session marker (written out, as now);
  - the usage-error cases;
  - the build and check hint cases;
  - where allowed.yaml may live, for the cases that need no answer;
  - the "real wiring" cases (R1.7 points them at ALZ_BIN instead of bundling src/cli/bin.ts).
- test/account-workspace.ts drops its `Terminal` import and parameter; the in-process variant goes to test/reference/in-process.ts.
- The PR lists every moved case. Fake-terminal cases are marked "back to acceptance in R1.8", which is the coverage the Rust oracle is missing until then.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No file under test/acceptance, and neither test/account-workspace.ts nor test/helpers.ts, imports src/cli/terminal, src/resolve/account-allow-writer, src/core/yaml or test/import-graph
- [x] #2 Every moved case runs under test/reference with its original name. The PR lists the fake-terminal cases as pending R1.8
- [x] #3 `mise run test` passes; no case name is lost and the combined acceptance plus reference count is at least the baseline, with per-project counts recorded in the notes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #3 reworded before start (2026-11-23) to match R1.3 to R1.5: per-project acceptance counts fall by design when cases move. R1.8 is deferred to R6 by the owner, so the fake-terminal cases stay in reference until then.

Implementation (test-only, src/ untouched).
- test/helpers.ts and test/account-workspace.ts no longer import src/cli/terminal and have no Terminal parameter. test/allow-driver.ts moved to test/reference/allow-driver.ts. In-process fake-terminal runs use layersInProcess (test/reference/in-process.ts).
- test/reference/32-account-allow.test.ts holds the whole former 32 under original names except two cases that stay acceptance-only (names every reason that applies; the real wiring cases) and uses snapshot from test/helpers.ts (the local copy is gone).
- test/reference/24-untrusted-repo.test.ts gained the three cases of "alz account allow with a repository that is not to be trusted".
- Guard: DIRECT is empty; 24 and 32 are in THROUGH_HELPERS.
- Known gap for R1.7: the real-wiring cases in acceptance 32 still bundle src/cli/bin.ts, from a path string, not an import, so the guard does not see it. R1.7 points them at ALZ_BIN.

Pending R1.8 (fake terminal, back to acceptance when a pseudo-terminal driver exists, deferred to R6): all of test/reference/32 except the trace/scan/writer cases below, and the three account-allow cases in test/reference/24.
Moved case by case (original name kept; counterpart in acceptance = CLI, no terminal):
- 32 who may run: "refuses `allow`/`allow --remove` without a terminal before any file system call" (trace; reference only) -> acceptance "refuses `%s` without a terminal and changes nothing". Marker cases "refuses with %s set ... even on a terminal, without asking" (terminal; pending R1.8) -> acceptance "... without a terminal, naming the marker only" for all six markers. "names every reason..." stays acceptance as is. Usage errors: original name in both (reference with a terminal, acceptance without).
- 32 prompt and what is written (all 11 cases), --remove (4), lock file (waits for a lock; writer-direct x2; twelve processes), layers decide (8), read again after confirmation (3), one reader (createTerminal x3), escaped not stripped, hint edge cases (8), hint cases that print the hint (terminal): reference only, pending R1.8 for those that need an answer or a terminal. No CLI counterpart: the refusal after the terminal check is unreachable without a terminal.
- 32 states refused (17 + --remove x5 + relative HOME + writer uid): reference; acceptance counterparts "shows %s as ignored ..." via account which (8 states that which prints) and "leaves %s untouched when alz account allow is run without a terminal" (all 13 states). Relative HOME is covered in 31. Writer uid: reference only (calls updateAllowed).
- 32 account applies after allow: reference; counterparts "starts a fresh clone on the default account until the repository is allowed", "keeps the approval across a ref bump, and skips the account when the map changes", "skips the account when a target is removed from the map, or the declaring source changes" (entry written by the test), "prints no allow hint from build and check without a terminal, ...", "prints no allow hint whatever allowed.yaml holds, ...".
- 32 where allowed.yaml may live: reference; counterparts for the git work tree cases (state directory; link placed in a work tree) via which. Link cases that need an answer: reference only.
- 32 trace case (writes only under the state directory), import-graph and standard-input scans (and their scan self-tests): reference only, no counterpart (TypeScript source scans; Rust has the capability types, R3).
- 24 three account-allow cases: "refuses a declaration the repository's own layer makes" is shown by the acceptance resolution case of the same file (warning, "inside the repository"); counterpart "prints no control character for a hub directory whose name holds ESC, from build and account which"; "shows the source ... before it asks" needs a person: reference only, pending R1.8.

Name coverage for 24 and 32 (vitest list, before 102 cases / 100 distinct names, after 115 / 112): no name lost, 12 new counterpart names.
Results: mise run lint passes (biome, tsc). Per project, run with --maxWorkers=2: acceptance 701 passed, 1 skipped (702; baseline 795 falls by design); reference 327 passed, 1 expected fail (328; baseline 176); combined 1030 vs baseline 971.
TASK-54 stall: two full `mise run test` runs failed only with TestTimeoutError (30 then 25 cases, in files this task did not touch too: 16, 22, 26, 28, 30, 31). The first run took 2194 s. Leftover alz-* folders were deleted (1929, 63, 1168 found at different times). Isolated runs of the touched files and both projects with --maxWorkers=2 pass. `mise run test` itself has not passed in one run.

Verified by the lead (2026-11-23): after deleting 1168 leftover alz-* folders, `mise run test` passed in one run: 42 files, 1028 passed, 1 expected fail, 1 skipped (1030), 73 s, no TestTimeoutError. `mise run lint` passes. Grep of test names in 24 and 32 (acceptance and reference, HEAD against working tree): 78 before, 86 after, none missing. The run left 2289 alz-* folders (about 950 before R1.6), mostly fixture copies from test/helpers.ts:26 that are never removed; R1.7's one temp root per run removes them (TASK-54).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Acceptance 24 and 32 no longer reach src/ directly: the fake-terminal, writer and source-scan cases moved to test/reference under their original names (fake-terminal ones pending R1.8, deferred to R6), with CLI counterparts added where alz shows the behaviour without a terminal. test/helpers.ts and test/account-workspace.ts lost their Terminal dependency, allow-driver moved to test/reference, and the guard's DIRECT list is empty. Counts: acceptance 702, reference 328, combined 1030 against a baseline of 971. Left for R1.7: the real-wiring cases in acceptance 32 still bundle src/cli/bin.ts by path.
<!-- SECTION:FINAL_SUMMARY:END -->
