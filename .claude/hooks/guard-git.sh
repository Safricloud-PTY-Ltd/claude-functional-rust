#!/usr/bin/env bash
# PreToolUse hook on every Bash command (git can hide behind cd, a pipe or a wrapper, so there
# is no `if` filter). The policy lives in git-policy.js: the main session is untouched, every
# sub-agent gets read-only git, and a nested architect also gets `add` and `commit` on files
# in its own territory. This wrapper fails closed: anything but a clean allow or a reasoned
# block is a block.
set -u

root="${CLAUDE_PROJECT_DIR:-$PWD}"
node "$root/.claude/hooks/git-policy.js"
status=$?
case "$status" in
  0|2) exit "$status" ;;
  *)   echo "guard-git: the git policy failed to run (exit $status); command blocked" >&2; exit 2 ;;
esac
