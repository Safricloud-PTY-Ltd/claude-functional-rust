#!/usr/bin/env bash
# Releases the file locks held by the given owners, and no others.
#
#   .claude/scripts/release-locks.sh <owner>...
#
# An owner is what a lock file in .claude/locks/ holds: a Claude sub-agent's id (the id its
# spawn returned), or codex-<brief id> for a Codex run. Releasing by owner, rather than
# clearing a whole territory or the whole folder, is what lets phases overlap: the next
# phase's test-writers can start while this phase's implementors still hold their files, and
# a brief's locks go as soon as it is verified. Prints each released path; exits 0 even when
# an owner held nothing.
set -u

(( $# > 0 )) || { echo "usage: $0 <owner>..." >&2; exit 64; }
root=$(cd "$(dirname "$0")/../.." && pwd)
locks="$root/.claude/locks"
[[ -d "$locks" ]] || exit 0

for lock in "$locks"/*; do
  [[ -f "$lock" ]] || continue
  holder=$(tr -d '\r\n' < "$lock")
  for owner in "$@"; do
    if [[ "$holder" == "$owner" ]]; then
      rm -f "$lock" && printf 'released %s (%s)\n' "$(basename "$lock" | sed 's#__#/#g')" "$owner"
      break
    fi
  done
done
exit 0
