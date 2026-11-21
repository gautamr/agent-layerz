#!/usr/bin/env bash
# Example check: the project's check passes, a commit was made, and the task reached In Review.
set -e
bash scripts/check.sh
test "$(git rev-list --count HEAD)" -gt 1
backlog task list --status "In Review" --plain | grep -q .
