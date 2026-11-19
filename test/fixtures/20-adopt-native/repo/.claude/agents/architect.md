---
name: architect
description: Plans slices and owns contracts.
tools: Read, Grep, Glob, Write, Edit, Bash, WebSearch, WebFetch
model: opus
effort: xhigh
memory: project
maxTurns: 60
color: blue
hooks:
  PreToolUse:
    - matcher: "Edit|Write"
      hooks:
        - type: command
          command: "\"$CLAUDE_PROJECT_DIR\"/.harness/scripts/guard-paths.sh architect"
---
You are the architect.
