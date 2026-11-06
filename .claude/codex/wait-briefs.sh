#!/usr/bin/env bash
# Waits for run-brief.sh runs to finish, and tells a run that died from one that is slow.
#
#   .claude/codex/wait-briefs.sh [--timeout <seconds>] <dir>/<id>.brief...
#
# A run has finished when its <id>.exit exists. It died when it has no <id>.exit and the
# process in <id>.pid is gone: its background task was killed, so nothing audited what Codex
# touched and its codex-<id> locks are still held. Read <id>.out and git status before
# anything else. A run with no live process is given 30 seconds before it counts as dead, so
# a wait started beside the runs doesn't misread a run that hasn't written its .pid yet.
#
# Prints one line per brief. Exits 0 when every run finished, 4 when any died (once the rest
# have finished), 3 when the timeout came first (default 540 s, under the Bash tool's 600 s
# cap: call it again), 64 on a usage error.
set -uo pipefail

usage() { echo "usage: $0 [--timeout <seconds>] <dir>/<id>.brief..." >&2; exit 64; }
timeout=540
if [[ "${1:-}" == --timeout ]]; then timeout="${2:-}"; shift 2 || usage; fi
[[ "$timeout" =~ ^[0-9]+$ ]] || usage
(( $# > 0 )) || usage
for b in "$@"; do [[ "$b" == *.brief ]] || usage; done

# finished, running or died. .exit is checked again after the process check, because a run
# that writes .exit and exits between the two checks has finished, not died.
state() {
  local base="$1" pid
  [[ -f "$base.exit" ]] && { echo finished; return; }
  pid=$(cat "$base.pid" 2>/dev/null)
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then echo running; return; fi
  [[ -f "$base.exit" ]] && { echo finished; return; }
  (( SECONDS < 30 )) && { echo running; return; }
  echo died
}

while :; do
  pending=0
  died=0
  lines=()
  for b in "$@"; do
    base="${b%.brief}"
    id=$(basename "$base")
    case "$(state "$base")" in
      finished) lines+=("$id: finished, exit $(cat "$base.exit")") ;;
      running)  lines+=("$id: running"); pending=1 ;;
      died)     lines+=("$id: DIED with no .exit; its process ($(cat "$base.pid" 2>/dev/null || echo 'never recorded')) is gone. Nothing audited what it touched: read $id.out and git status, and release its codex-$id locks.")
                died=1 ;;
    esac
  done
  (( pending == 0 || SECONDS >= timeout )) && break
  sleep 10
done

printf '%s\n' "${lines[@]}"
(( pending )) && exit 3
(( died )) && exit 4
exit 0
