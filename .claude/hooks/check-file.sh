#!/usr/bin/env bash
# PostToolUse hook on Edit|Write. After a .rs file changes: format-check the file, lint its
# crate, run its tests. Exit 2 hands the output back to the agent; the edit already happened.
#
#   rustfmt     the one file, --check. Never rewrites; the agent fixes what it wrote.
#   clippy      the file's crate, all targets, warnings are errors. This is also the
#               typecheck. Sub-agents see only the diagnostics in the file they touched: an
#               error elsewhere is a sibling mid-write, or the architect's problem at phase
#               verification.
#   tests       the file's own tests (<name>::tests::, or the tests/<name>.rs target).
#               Skipped for stubs (red by design), lib.rs / mod.rs / types.rs (everything
#               uses them), and for the test-writer (its tests are meant to be red; it runs
#               them itself). Uses cargo-nextest when installed, cargo test otherwise.
#
# Parallel agents share one target/ directory, so cargo serialises them on its build lock.
# That is slower than it could be and always correct. Not a policy hook, so it fails open.
set -u

norm() {
  local p="${1//\\//}"
  if command -v cygpath >/dev/null 2>&1; then cygpath -u "$p"; else printf '%s\n' "$p"; fi
}

root=$(norm "${CLAUDE_PROJECT_DIR:-$PWD}")
fields=$(node "$root/.claude/hooks/hook-input.js" tool_input.file_path agent_id agent_type 2>/dev/null) || exit 0
{ read -r path; read -r agent_id; read -r agent_type; } <<<"$fields"

case "$path" in *.rs) ;; *) exit 0 ;; esac
# No workspace yet (the scaffold isn't installed), so there's nothing to lint or test.
[[ -f "$root/Cargo.toml" ]] || exit 0
path=$(norm "$path")
[[ -f "$path" ]] || exit 0
rel="${path#"$root"/}"
cd "$root" || exit 0

# The crate is the nearest ancestor with a Cargo.toml that isn't the workspace root.
dir=$(dirname "$path")
while [[ "$dir" != "$root" && "$dir" != "/" && ! -f "$dir/Cargo.toml" ]]; do dir=$(dirname "$dir"); done
[[ "$dir" == "$root" || ! -f "$dir/Cargo.toml" ]] && exit 0
pkg=$(sed -n 's/^name *= *"\(.*\)"/\1/p' "$dir/Cargo.toml" | head -n 1)
[[ -n "$pkg" ]] || exit 0

report=""
failed=0
section() { report+="== $1"$'\n'"$2"$'\n'; failed=1; }

# 1. Format
if ! out=$(rustfmt --check --edition 2024 -- "$path" 2>&1); then
  section "rustfmt --check (run: rustfmt --edition 2024 $rel)" "$(head -n 60 <<<"$out")"
fi

# 2. Lint, which is also the typecheck
if ! out=$(cargo clippy --quiet -p "$pkg" --all-targets --message-format=short -- -D warnings 2>&1); then
  out=$(tr '\\' '/' <<<"$out")
  [[ -n "$agent_id" ]] && out=$(grep -F -- "$rel" <<<"$out" || true)
  [[ -n "$out" ]] && section "clippy -p $pkg" "$(head -n 80 <<<"$out")"
fi

# 3. Tests
filter=()
stem=$(basename "$rel" .rs)
case "$rel" in
  */tests/*.rs)                    filter=(--test "$stem") ;;
  */lib.rs|*/mod.rs|*/types.rs|*/main.rs) ;;
  *_tests.rs)                      filter=(-- "${stem%_tests}::tests::") ;;
  *)                               filter=(-- "${stem}::tests::") ;;
esac
grep -q '@stub' "$path" && filter=()
[[ "$agent_type" == test-writer ]] && filter=()

if (( ${#filter[@]} )) && (( ! failed )); then
  if cargo nextest --version >/dev/null 2>&1; then
    [[ "${filter[0]}" == "--" ]] && filter=("${filter[@]:1}")
    runner=(cargo nextest run -p "$pkg" --no-tests=pass "${filter[@]}")
  else
    runner=(cargo test --quiet -p "$pkg" "${filter[@]}")
  fi
  if ! out=$("${runner[@]}" 2>&1); then
    section "${runner[*]}" "$(tail -n 80 <<<"$out")"
  fi
fi

if (( failed )); then
  printf '%s' "$report" >&2
  exit 2
fi
exit 0
