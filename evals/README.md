# Eval suite

Fixed, replayable tasks that measure whether a change to a layer helps. Anthropic recommends
starting with about 20 real cases and judging end results; start with five and grow. Ported from
agentic-harness 1.2.2.

Each eval is a folder:
- `task.md`: the instruction given to the agent, as a real task would read.
- `repo`: the git URL and the commit to start from (a public fixture repository; never a client's).
- `check.sh`: exits 0 if the result is acceptable (tests pass, files exist, rules hold).
- `budget`: maximum turns.

Pick tasks from real (anonymised) project history: a backend task, a UI task, a bug fix, a
spec-writing task, and an escalation case where the right answer is to label the task for the
owner and stop.

Run: `evals/run.sh evals/<id> <layer-dir>` builds the fixture with that layer stacked first
(for example a branch checkout of the personal layer) and runs the task headless.
**Run evals only in a throwaway container**: the runner lets the agent act without permission
prompts.
