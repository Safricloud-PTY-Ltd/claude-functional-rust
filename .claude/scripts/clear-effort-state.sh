#!/usr/bin/env bash
# Removes an effort's runtime state: .claude/territories (the orchestrator's territory files)
# and .claude/locks (the write guard's and run-brief.sh's per-file locks).
#
# Run it by hand, never from a hook. Several Claude Code sessions can share this checkout,
# and a session opened to ask a question would otherwise wipe the state of an effort another
# session is running. Who runs it, and when:
#   - the orchestrator, after the merge (orchestrator.md, step 11);
#   - the orchestrator, at the start of an effort, on leftovers the owner has confirmed belong
#     to no running effort (orchestrator.md, step 1);
#   - a solo architect, after its merge (architect.md, "Dispatch a phase at a time").
#
# Prints each directory it removed, or that there was nothing to remove.
set -euo pipefail

root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
removed=0
for dir in .claude/territories .claude/locks; do
  if [[ -e "$root/$dir" ]]; then
    rm -rf "${root:?}/$dir"
    printf 'removed %s\n' "$dir"
    removed=1
  fi
done
(( removed )) || printf 'nothing to remove: no .claude/territories or .claude/locks\n'
