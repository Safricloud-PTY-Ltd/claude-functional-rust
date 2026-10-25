#!/usr/bin/env bash
# PreToolUse hook on Edit|Write. Decides who may write which file.
#
#   main session (no agent_id)  unrestricted: the orchestrator, or a solo architect
#   architect (a sub-agent)     only inside the territory that names its agent id, and inside
#                               no other territory
#   test-writer                 only <name>_tests.rs beside an existing <name>.rs contract, or
#                               <crate>/tests/*.rs in a crate that has a src/lib.rs
#   implementor                 only existing .rs files under a crate's src/ that carry an
#                               @stub marker, or a file it already holds; never tests, never
#                               new files, never Cargo.toml
#   test-writer, implementor    also inside exactly one territory, whenever any territory exists
#   any file                    at most one sub-agent owner per phase (.claude/locks; an
#                               architect releases its own territory's locks before each
#                               phase, SessionStart clears them all)
#
# Territories, and the canonical repo-relative path (.. and symlinks resolved), come from
# territory.js, which the git guard shares.
#
# Exit 2 blocks the write and hands the message to the agent as the reason.
# Fails closed: if the input can't be parsed, the write is blocked.
set -u

# Windows hands us C:\a\b; bash wants /c/a/b. Same path, one spelling, before any comparison.
norm() {
  local p="${1//\\//}"
  if command -v cygpath >/dev/null 2>&1; then cygpath -u "$p"; else printf '%s\n' "$p"; fi
}

root=$(norm "${CLAUDE_PROJECT_DIR:-$PWD}")
fields=$(node "$root/.claude/hooks/hook-input.js" agent_id agent_type tool_input.file_path 2>/dev/null) \
  || { echo "guard-writes: could not parse hook input; write blocked" >&2; exit 2; }
{ read -r agent_id; read -r agent_type; read -r path; } <<<"$fields"

[[ -z "$agent_id" || -z "$path" ]] && exit 0
case "$agent_type" in
  architect|implementor|test-writer) ;;
  *) exit 0 ;;   # Explore and other sub-agents follow the normal permission flow
esac

deny() { printf 'guard-writes: %s\n' "$1" >&2; exit 2; }

verdict=$(node "$root/.claude/hooks/territory.js" "$agent_id" "$agent_type" "$path" 2>/dev/null) \
  || deny "could not read the territories; write blocked."
{ read -r rel; read -r why; } <<<"$verdict"
[[ "$why" == ok ]] || deny "$why"
[[ "$agent_type" == architect ]] && exit 0
[[ -n "$rel" ]] || deny "$agent_type may only write inside the repository (asked for $path)."
path="$root/$rel"

# Which crate, and which part of it.
case "$rel" in
  crates/*/src/*.rs|crates/*/tests/*.rs) crate="${rel#crates/}"; crate="crates/${crate%%/*}" ;;
  xtask/src/*.rs|xtask/tests/*.rs)       crate="xtask" ;;
  *) deny "$agent_type may only write .rs files under a crate's src/ or tests/ (asked for $rel). Put the need in your report." ;;
esac
inner="${rel#"$crate"/}"

is_test=0
case "$inner" in
  tests/*)         is_test=1 ;;
  src/*_tests.rs)  is_test=1 ;;
esac

if [[ "$agent_type" == test-writer ]]; then
  (( is_test )) || deny "test-writer writes only <name>_tests.rs and <crate>/tests/*.rs files (asked for $rel)."
  case "$inner" in
    tests/*/*) deny "test-writer writes tests/*.rs, not files below it (asked for $rel). Fixtures are the architect's." ;;
    tests/*)   [[ -f "$root/$crate/src/lib.rs" ]] || deny "no src/lib.rs in $crate, so there is no public API to test yet." ;;
    *)         contract="${rel%_tests.rs}.rs"
               [[ -f "$root/$contract" ]] || deny "no contract at $contract for $rel. Tests go beside an existing contract; report CLARIFY if the brief points elsewhere." ;;
  esac
else
  (( is_test )) && deny "implementor does not edit tests (asked for $rel). A test that contradicts the contract is a BLOCKED report."
  [[ -f "$path" ]] || deny "implementor does not create files (asked for $rel). Report the function you need under Needs."
fi

lockdir="$root/.claude/locks"
mkdir -p "$lockdir"
lock="$lockdir/${rel//\//__}"

if [[ -f "$lock" ]]; then
  [[ "$(cat "$lock")" == "$agent_id" ]] || deny "$rel is owned by another agent this phase. Report the overlap under Blocked on."
  exit 0
fi

if [[ "$agent_type" == implementor ]] && ! grep -q '@stub' "$path"; then
  deny "$rel has no @stub marker, so it is not open for implementation. Implemented files, lib.rs, mod.rs and types.rs are the architect's."
fi

# Created atomically (noclobber opens with O_EXCL), as run-brief.sh does, so a racing agent
# or Codex run can't both take the file.
( set -C; printf '%s' "$agent_id" > "$lock" ) 2>/dev/null && exit 0
[[ "$(cat "$lock" 2>/dev/null)" == "$agent_id" ]] || deny "$rel is owned by another agent this phase. Report the overlap under Blocked on."
exit 0
