#!/usr/bin/env bash
# Requests one review round on a pull request, as "Requesting a review" in CONTRIBUTING.md
# describes.
#
#   .claude/scripts/request-review.sh <pr number> <codex|claude>
#
# Reads the reviewer's model, effort and speed from .claude/settings.json env (CLAUDE.md,
# Models), and refuses a Codex model outside CODEX_ALLOWED_MODELS or an unknown speed before
# any git or gh work. Counts the reviewer's markers on the PR and stops at 9. Checks that the
# checkout this script lives in is exactly the PR head with a clean tree: the reviewer reads
# that checkout. To review while agents are still writing, run the copy of this script inside
# a worktree at the PR head instead (CONTRIBUTING.md). Runs the reviewer in the foreground,
# then confirms its round's marker landed. With PR reviewer `both`, run one copy per reviewer
# at the same time.
#
# Exit 0: the round posted. 1: a precondition or the run failed. 3: the cap is reached.
# 64: bad arguments or settings.
set -u

usage() { echo "usage: request-review.sh <pr number> <codex|claude>" >&2; exit 64; }
[[ $# -eq 2 ]] || usage
N="$1"; R="$2"
[[ "$N" =~ ^[0-9]+$ ]] || usage
case "$R" in codex|claude) ;; *) usage ;; esac
fail() { echo "request-review: $1" >&2; exit "${2:-1}"; }

case "$R" in
  codex)  model="${CODEX_REVIEW_MODEL:-gpt-6-astra}"; effort="${CODEX_REVIEW_EFFORT:-medium}"; speed="${CODEX_REVIEW_SPEED:-base}" ;;
  claude) model="${CLAUDE_REVIEW_MODEL:-opus}"; effort="${CLAUDE_REVIEW_EFFORT:-high}"; speed="" ;;
esac
if [[ "$R" == codex && -n "${CODEX_ALLOWED_MODELS:-}" && ",${CODEX_ALLOWED_MODELS// /}," != *",$model,"* ]]; then
  fail "codex model $model isn't allowed; CODEX_ALLOWED_MODELS (.claude/settings.json env) is $CODEX_ALLOWED_MODELS" 64
fi
# Codex accepts an unknown service tier silently, so only these three words are allowed.
tier=()
case "$speed" in
  ''|base)   ;;
  fast)      tier=(-c service_tier=priority) ;;
  ultrafast) tier=(-c service_tier=ultrafast) ;;
  *) fail "CODEX_REVIEW_SPEED is '$speed', not base, fast or ultrafast" 64 ;;
esac

# The project root is where this script is installed (<project>/.claude/scripts/), not git's
# top level: a project nested in a larger repository reviews from the project directory.
# Enter it before the first gh call, since gh infers the repository from the working directory.
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd) || fail "could not resolve the project root"
cd "$root" || fail "could not enter $root"

count() {
  gh pr view "$N" --json reviews,comments \
    --jq "[.reviews[].body, .comments[].body] | map(select(startswith(\"<!-- ai-review: $R \"))) | length"
}
before=$(count) || fail "could not read PR #$N"
[[ "$before" =~ ^[0-9]+$ ]] || fail "could not count $R's reviews on PR #$N (got '$before')"
(( before < 9 )) || fail "$R has reviewed PR #$N 9 times; the cap is reached" 3
round=$((before + 1))
head=$(gh pr view "$N" --json headRefOid -q .headRefOid) || fail "could not read PR #$N's head"

at=$(git rev-parse HEAD) || fail "$root is not a git checkout"
[[ "$at" == "$head" ]] || fail "the checkout at $root is at $at, not the PR head $head. Commit and push first, or run this from a worktree at the PR head."
dirty=$(git status --porcelain)
[[ -z "$dirty" ]] || fail "the working tree at $root differs from the PR head, and the reviewer reads it. Commit and push first, or run this from a worktree at the PR head:
$dirty"

# The review payload goes through a file, never stdin or a here-document. target/ is gitignored.
payload="target/review/pr$N-$R-$round.json"
mkdir -p target/review || fail "could not create target/review"
rm -f "$payload"   # a failed earlier attempt at this round would otherwise be read back in
prompt="You are the $R reviewer, running $model at $effort effort${tier[*]:+, $speed speed}; your review header names it as \`$model ($effort)\`. Write your review payload to $payload and post it from there. Review pull request #$N as review $round of 9 by following \"Doing a review\" in CONTRIBUTING.md exactly."
echo "request-review: $R round $round/9 on PR #$N at ${head:0:7}, from $root ($model, $effort${tier[*]:+, $speed})"

case "$R" in
  codex)
    # Inside Codex's sandbox gh can't reach the system keyring, so the token goes in for this run.
    token=$(gh auth token) || fail "gh auth token failed; run gh auth login"
    printf '%s\n' "$prompt" |
      GH_TOKEN="$token" codex exec -m "$model" -c "model_reasoning_effort=$effort" ${tier[@]+"${tier[@]}"} \
        -s workspace-write -c sandbox_workspace_write.network_access=true \
        -c shell_environment_policy.inherit=all -c shell_environment_policy.ignore_default_excludes=true -
    ;;
  claude)
    printf '%s\n' "$prompt" |
      claude -p --model "$model" --effort "$effort" --setting-sources user \
        --permission-mode dontAsk --no-session-persistence \
        --allowedTools Read Grep Glob "Edit(target/review/**)" "Bash(git status:*)" \
          "Bash(git diff:*)" "Bash(git log:*)" "Bash(git show:*)" "Bash(git rev-parse:*)" \
          "Bash(gh pr view:*)" "Bash(gh pr diff:*)" "Bash(gh api:*)" "Bash(gh pr comment:*)" \
          "Bash(cargo nextest run:*)"
    ;;
esac
status=$?

after=$(count) || fail "the reviewer exited ($status), and PR #$N couldn't be re-read to confirm the round"
(( after > before )) || fail "the reviewer exited ($status) without posting round $round. Read its output above, fix the cause, and request again; a failed run doesn't count against the 9."
echo "request-review: $R review $round/9 posted on PR #$N"
