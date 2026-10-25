#!/usr/bin/env bash
# Runs one test-writer (T-…) or implementor (I-…) brief on Codex and checks what it touched.
# The write guard and the file locks in .claude/hooks/ never see Codex's writes, so this
# script stands in for them. It takes the locks for the brief's Write: paths before the run.
# After the run it lists every path whose content changed and every one the brief didn't name.
#
#   .claude/codex/run-brief.sh <dir>/<id>.brief [medium|high]
#   .claude/codex/run-brief.sh --resume <dir>/<id>.brief <answer-file> [medium|high]
#
# The brief file is named after its id (T-app-5.3.brief) and carries one line
#   Write: <repo-relative path>[, <path>...]       or  Write: none
# naming every file the brief may write. The script reads the role from the id's first
# letter and puts the role header in front of the brief.
#
# Beside the brief it writes:
#   <id>.prompt   header + brief, as sent          <id>.out      Codex's full log (appended)
#   <id>.report   Codex's last message, the report <id>.touched  paths whose content changed
#   <id>.stray    touched paths not on Write:      <id>.exit     exit status, written last
# and, for its own use, <id>.allowed, <id>.before and <id>.after, rewritten every run.
# A usage error before the brief file is found writes no <id>.exit at all: read the output.
# Wait on <id>.exit. Its status is 0 (clean), 3 (a lock is held by someone else, nothing
# ran), 4 (stray paths), 64 (usage), 65 (the tree couldn't be snapshotted or compared, so
# what the run touched is unknown), or 70 (Codex itself exited non-zero; the output's first
# line has its status, and .touched and .stray are still written).
#
# The touched list compares content hashes of every modified or untracked, non-ignored file
# before and after the run, so it also catches an edit to a file that was already dirty.
# Other agents work in the same checkout, so a sibling's edit made during the run shows up
# too. Tell the two apart with the locks and git diff before undoing anything.
#
# --resume continues the same Codex session (its id is read from <id>.out) with the answer
# file as the next prompt, for a NEEDS, CLARIFY or BLOCKED report. Edit the brief's Write:
# line first if the answer widens it.
set -uo pipefail

model="${RUN_BRIEF_MODEL:-gpt-6-astra}"

usage() { echo "usage: $0 [--resume] <dir>/<id>.brief [answer-file] [medium|high]" >&2; exit 64; }

resume=0
[[ "${1:-}" == --resume ]] && { resume=1; shift; }
# Without a brief file there is no <id> to write a status for: plain exit 64.
brief="${1:-}"; [[ -n "$brief" && -f "$brief" ]] || usage

abs() { (cd "$(dirname "$1")" && printf '%s/%s\n' "$(pwd)" "$(basename "$1")"); }
brief=$(abs "$brief")
dir=$(dirname "$brief")
id=$(basename "$brief" .brief)
base="$dir/$id"

# From here on, every exit writes a fresh <id>.exit, so a waiter never reads an earlier
# run's status or report.
rm -f "$base.exit" "$base.report" "$base.touched" "$base.stray"
finish() { printf '%s\n' "$1" > "$base.exit"; exit "$1"; }
bad_usage() { echo "usage: $0 [--resume] <dir>/<id>.brief [answer-file] [medium|high]" >&2; finish 64; }

answer=""
if (( resume )); then answer="${2:-}"; [[ -n "$answer" && -f "$answer" ]] || bad_usage; shift; fi
effort="${2:-medium}"
case "$effort" in medium|high) ;; *) bad_usage ;; esac
[[ -n "$answer" ]] && answer=$(abs "$answer")
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root" || finish 64

# Speed: RUN_BRIEF_SPEED, else CLAUDE.md's **Codex speed** setting. fast and ultrafast are
# Codex's `priority` and `ultrafast` service tiers; base sends none. Codex accepts any tier
# name silently, so an unknown speed is refused here.
speed="${RUN_BRIEF_SPEED:-$(sed -n 's/^- \*\*Codex speed:\*\* `\([a-z]*\)`.*/\1/p' CLAUDE.md | head -n 1)}"
case "$speed" in
  base)      tier=() ;;
  fast)      tier=(-c service_tier=priority) ;;
  ultrafast) tier=(-c service_tier=ultrafast) ;;
  *) echo "run-brief: Codex speed '$speed' is not base, fast or ultrafast" >&2; finish 64 ;;
esac

case "$id" in
  T-*) role=test-writer ;;
  I-*) role=implementor ;;
  *)   echo "run-brief: $id is neither a T- nor an I- brief id" >&2; finish 64 ;;
esac

# The Write: paths, one per line. A brief written on Windows may be CRLF; a kept \r would
# name a different lock than guard-writes.sh does and never match .touched.
line=$(grep -m1 '^Write:' "$brief" | tr -d '\r' | sed 's/^Write:[[:space:]]*//')
[[ -n "$line" ]] || { echo "run-brief: $brief has no Write: line" >&2; finish 64; }
writes=()
if [[ "$line" != none ]]; then
  # Every word is taken as a path, so prose on this line would lock files named after
  # words. Globbing is off for the split, so a `*` in a word can't expand into the
  # checkout's files.
  set -f
  for p in $(tr ',' ' ' <<<"$line"); do
    case "$p" in /*|*..*|*\\*) echo "run-brief: Write: path $p must be repo-relative, with /" >&2; finish 64 ;; esac
    case "$p" in
      */*) ;;
      *) echo "run-brief: Write: word '$p' isn't a path (no /). Keep Write: to bare paths and put prose on the next line" >&2; finish 64 ;;
    esac
    writes+=("$p")
  done
  set +f
fi
printf '%s\n' ${writes[@]+"${writes[@]}"} > "$base.allowed"

# Locks: the same files guard-writes.sh uses, so no Claude sub-agent takes these paths this
# phase. The architect's own per-phase release frees them. Each is created atomically
# (noclobber opens with O_EXCL), so two runs racing for one path can't both take it; a lock
# already ours (a resume) is kept, and on a conflict the locks this run took are released.
mkdir -p .claude/locks || finish 65
owner="codex-$id"
taken=()
release_taken() { rm -f ${taken[@]+"${taken[@]}"}; }
for p in ${writes[@]+"${writes[@]}"}; do
  lock=".claude/locks/${p//\//__}"
  if ( set -C; printf '%s' "$owner" > "$lock" ) 2>/dev/null; then
    taken+=("$lock")
  elif [[ "$(cat "$lock" 2>/dev/null)" != "$owner" ]]; then
    echo "run-brief: $p is held by $(cat "$lock" 2>/dev/null) this phase; nothing ran" >&2
    release_taken
    finish 3
  fi
done

# Writes to $1 one line per modified or untracked path: its content hash, or `deleted` for a
# tracked path that no longer exists, so a run that deletes a tracked file still shows as
# touched. Returns non-zero if the tree can't be listed or a file can't be hashed, so a run
# whose audit is unavailable never reports clean.
snapshot() {
  local out="$1" p sum
  git ls-files -z --modified --others --exclude-standard > "$out.z" || return 1
  while IFS= read -r -d '' p; do
    if [[ -e "$p" ]]; then
      sum=$(sha1sum -- "$p") || return 1
      printf '%s %s\n' "${sum%% *}" "$p"
    else
      printf 'deleted %s\n' "$p"
    fi
  done < "$out.z" > "$out.raw" || return 1
  LC_ALL=C sort -u "$out.raw" > "$out" || return 1
  rm -f "$out.z" "$out.raw"
}
audit_failed() { echo "run-brief: could not audit what the run touched ($1); check git status yourself" >&2; finish 65; }

snapshot "$base.before" || { release_taken; echo "run-brief: could not snapshot the tree; nothing ran" >&2; finish 65; }

if (( resume )); then
  sid=$(sed -n 's/^session id:[[:space:]]*\([0-9a-fA-F-]*\).*/\1/p' "$base.out" 2>/dev/null | head -n 1)
  [[ -n "$sid" ]] || { echo "run-brief: no session id in $base.out to resume" >&2; finish 64; }
  printf '\n===== resume %s with %s\n' "$sid" "$answer" >> "$base.out"
  codex exec resume -m "$model" -c model_reasoning_effort="$effort" ${tier[@]+"${tier[@]}"} \
    -c sandbox_mode=workspace-write -c shell_environment_policy.inherit=all \
    -o "$base.report" "$sid" - < "$answer" >> "$base.out" 2>&1
  status=$?
else
  {
    printf 'You are %s, the %s for one brief in this repo. Read AGENTS.md, then\n' "$id" "$role"
    printf '.claude/agents/%s.md, and follow that file as your role, with the brief below.\n' "$role"
    printf 'Write only the files on the brief'"'"'s Write: line. Create no other file. Run no git\n'
    printf 'command that changes state, and never `cargo fmt --all`, `cargo clean` or\n'
    printf '`cargo xtask check`. Validate only with the brief'"'"'s own commands. End with the\n'
    printf 'report format from your role file, as your last message.\n\n'
    cat "$brief"
  } > "$base.prompt"
  : > "$base.out"
  codex exec -m "$model" -c model_reasoning_effort="$effort" ${tier[@]+"${tier[@]}"} -s workspace-write \
    -c shell_environment_policy.inherit=all -C "$root" \
    -o "$base.report" - < "$base.prompt" >> "$base.out" 2>&1
  status=$?
fi

snapshot "$base.after" || audit_failed "snapshot"
LC_ALL=C comm -3 "$base.before" "$base.after" | sed 's/^\t//' | cut -d' ' -f2- \
  | LC_ALL=C sort -u > "$base.touched" || audit_failed "compare"
grep -vxF -f "$base.allowed" "$base.touched" > "$base.stray"
(( $? <= 1 )) || audit_failed "stray list"

shown="$base"
command -v cygpath >/dev/null 2>&1 && shown=$(cygpath -m "$base")
echo "$id: codex exited $status ($role, $effort, $speed)"
echo "report: $shown.report"
echo "touched ($(wc -l < "$base.touched")):"; sed 's/^/  /' "$base.touched"
if [[ -s "$base.stray" ]]; then
  echo "STRAY, not on Write: ($(wc -l < "$base.stray")). A path another agent holds a lock on is almost"
  echo "certainly that agent's; check the rest with git diff."
  while IFS= read -r p; do
    lock=".claude/locks/${p//\//__}"
    if [[ -f "$lock" ]]; then echo "  $p  [locked by $(cat "$lock")]"; else echo "  $p"; fi
  done < "$base.stray"
fi
echo "-----"
if [[ -s "$base.report" ]]; then cat "$base.report"; echo; else echo "(no report: read $shown.out)"; fi

# Codex's own status is on the first line above; 70 keeps it from posing as 3, 4, 64 or 65.
(( status != 0 )) && finish 70
[[ -s "$base.stray" ]] && finish 4
finish 0
