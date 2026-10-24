# Contributing

## Disk: target/ and scratch builds

Under a dozen parallel agents, `target/` once reached 300 GB for a 2 MB binary: full debug
info, the incremental cache, stale build variants and a coverage copy. The root `Cargo.toml`
gives dev and test builds line tables only (enough for panics and backtraces). For a debugger
session, set `CARGO_PROFILE_DEV_DEBUG=full` for that one build.

It still grows, because cargo never deletes stale build variants. So:

- **Clean at rest.** Run `cargo clean` after every merge, at every pause, and whenever
  `du -sh target` passes 40 GB while nothing is building. Never clean while agents are
  building: it pulls files out from under them.
- **Coverage builds are temporary.** `cargo llvm-cov` keeps its own copy in
  `target/llvm-cov-target`. Run `cargo llvm-cov clean --workspace` when you're done measuring.
- **Scratch builds.** An agent that builds in a scratch copy of the workspace uses one target
  directory for its whole workstream, never one per sub-agent, sets `CARGO_INCREMENTAL=0`
  there, and deletes the copy and its target when the phase ends.
- **Editors share it.** rust-analyzer's check writes into `target/` too. That's fine, since it
  uses the same profile, but it rebuilds as soon as `target/` is cleaned.

## Pull request review

The **PR reviewer** setting in `CLAUDE.md` decides whether this section applies. With `none`,
skip it: a PR goes from draft to ready to merge. With `codex` or `claude`, pull requests are
reviewed by that model on request, never automatically.

| Reviewer | Model | How it starts |
|---|---|---|
| `codex` | GPT-6 Astra, `medium` effort, via the Codex CLI | By request, as below. |
| `claude` | Claude Opus, `high` effort, via Claude Code headless | By request, as below; also the fallback when Codex can't run. |

The model names and flags below were measured on 2026-09-22. Update them here when you change
them.

**A PR is opened as a draft and stays one until its review loop is complete.** CI skips
drafts. Marking the PR ready (`gh pr ready <n>`) starts CI, so it runs once per PR and not
once per review round.

Requested reviews run on the owner's own subscriptions (ChatGPT for Codex, Claude for Claude
Code), not on API keys. Each reviewer posts its own review to the PR with the `gh` CLI.

### Requesting a review

Request one when the PR is opened, and again after every push that changes it. Each reviewer
reviews a PR **at most 9 times**. Every review's first line is a marker,
`<!-- ai-review: <reviewer> <n>/9 -->`, and the count is the number of markers already on the
PR:

```bash
N=12            # the PR number
R=codex         # or claude
DONE=$(gh pr view "$N" --json reviews,comments \
  --jq "[.reviews[].body, .comments[].body] | map(select(startswith(\"<!-- ai-review: $R \"))) | length")
ROUND=$((DONE + 1))   # stop if DONE is already 9
```

The reviewer reads the checkout, so the checkout must be exactly the PR's head: commit
everything, push, then check that both lines below print nothing:

```bash
test "$(git rev-parse HEAD)" = "$(gh pr view "$N" --json headRefOid -q .headRefOid)" || echo "checkout is not the PR head"
git status --porcelain
```

While agents are still working in the checkout, review from a worktree at the PR head
instead: `git worktree add --detach <scratchpad>/review-wt <head>`, run the reviewer there with
its own `CARGO_TARGET_DIR`, then `rm -rf` it and `git worktree prune`. (`git worktree remove`
fails on Windows' path-length limit once `target/` exists.)

**Codex.** Inside Codex's sandbox, `gh` can't reach the system keyring, so the token goes in
as `GH_TOKEN` for this one run. `workspace-write` with network on is the least access under
which posting works.

```bash
printf 'You are the codex reviewer, running GPT-6 Astra at medium effort. Review pull request #%s as review %s of 9 by following "Doing a review" in CONTRIBUTING.md exactly.\n' "$N" "$ROUND" |
  GH_TOKEN="$(gh auth token)" codex exec -m gpt-6-astra -c model_reasoning_effort=medium \
    -s workspace-write -c sandbox_workspace_write.network_access=true \
    -c shell_environment_policy.inherit=all -c shell_environment_policy.ignore_default_excludes=true -
```

**Claude.** `--setting-sources user` keeps this repo's `agent: orchestrator` default and its hooks
out of the reviewer's session. `dontAsk` refuses anything not on the allow-list, so the
reviewer can read, run a crate's tests, and talk to GitHub, but can't edit files or run git
commands that change state.

```bash
printf 'You are the claude reviewer, running Claude Opus at high effort. Review pull request #%s as review %s of 9 by following "Doing a review" in CONTRIBUTING.md exactly.\n' "$N" "$ROUND" |
  claude -p --model opus --effort high --setting-sources user \
    --permission-mode dontAsk --no-session-persistence \
    --allowedTools Read Grep Glob "Bash(git diff:*)" "Bash(git log:*)" "Bash(git show:*)" \
      "Bash(git rev-parse:*)" "Bash(gh pr view:*)" "Bash(gh pr diff:*)" "Bash(gh api:*)" \
      "Bash(gh pr comment:*)" "Bash(cargo nextest run:*)"
```

A review takes minutes, so run it in the background. When it exits, check that the round's
marker is now on the PR. A run that exits without posting didn't review anything: read its
output, fix the cause, and request again. A failed run doesn't count against the 9.

On a large diff, each round says it isn't exhaustive. Aim each request at what the last
rounds skipped (production paths, then tests and tables, then prose and fidelity), and reply
to every thread before the next round, which reads the replies.

### Doing a review

This part is for the reviewing model. Your request told you which reviewer you are (`codex`
or `claude`), the PR number `N`, and your round `n`.

**Post exactly one review per run, whatever you find.** A clean review is still a review:
post it, and say what you checked.

1. **Confirm what you're reviewing.** `gh pr view N --json number,title,body,baseRefName,headRefName,headRefOid`
   and `git rev-parse HEAD`. If they differ, don't review. Post a review that says the
   checkout was at `<HEAD>` and not the PR head `<headRefOid>`, and stop. Then run
   `git status --porcelain`. If it prints anything, the files you would read and test aren't
   the PR's. Don't review: post a review that lists those paths and says the working tree
   differs from the PR, and stop. Never discard, stash or commit someone else's changes.
2. **Read, in this order:**
   - the PR title and body;
   - the diff: `gh pr diff N`, or `git diff <baseRefName>...HEAD`;
   - `CLAUDE.md`, and `.claude/skills/code-standards/SKILL.md` in full, especially its
     **Review checklist**;
   - the contribution's `review.md` and `plan.md`, in `contributions/complete/<id>/` or
     `contributions/in-progress/<id>/`, where `<id>` is the branch name without its
     `feat/`, `fix/` or `chore/` prefix;
   - every file the diff touches, in full, and the definitions of whatever the changed code
     calls;
   - earlier rounds on this PR and the replies to them (`gh pr view N --comments`, and
     `gh api repos/{owner}/{repo}/pulls/N/comments` for inline threads), so you don't
     repeat a finding that was answered unless the answer is wrong.
3. **Judge, in priority order:**
   1. **Correctness.** Wrong results, a panic path in production code (`unwrap`, indexing,
      overflow, slicing), errors swallowed or mislabelled, off-by-one, code that disagrees
      with the spec it cites (under `docs/`, where the project keeps them), hostile input
      (allocation sized from input, unbounded recursion, stale state after an error), and a
      change that is right in isolation but breaks a caller elsewhere.
   2. **Tests.** Does each test pin the behaviour its contract's doc comment promises? Would
      it fail if the body were wrong? Are the edge cases and properties the contract implies
      covered? Is any test helper copied rather than imported?
   3. **Standards.** Every item on the code-standards **Review checklist**, and `CLAUDE.md`'s
      rules.
   4. **Fidelity.** Does the diff do what the PR body and `plan.md` say, and nothing they
      don't? A departure from the plan that isn't recorded under **Deviations** or
      **Decisions made mid-loop** is a finding. The plan is frozen once the contribution is
      archived to `contributions/complete/`, before the PR opens. From then on, a change made
      in answer to a review is recorded in the PR body's **Decisions to veto** and in the
      reply to the finding, never in `plan.md`. Check it there, and don't ask for it in the
      plan.
   5. **Prose.** Doc comments, `CLAUDE.md`, `CONTRIBUTING.md` and the PR body say what the
      code actually does.

   Don't flag formatting or lints that `cargo xtask check` enforces, or matters of taste no
   rule covers. You may run `cargo nextest run -p <crate>` on a crate the diff touches to
   confirm a claim. Don't run the full check or a release build.

   A finding carries a severity: **P0** (breaks the build, loses data, or is unsafe),
   **P1** (wrong behaviour), **P2** (a rule broken or a test that proves nothing), or **P3**
   (worth doing, not blocking). It gives the evidence, meaning the rule it breaks or the
   input that fails, and the fix.
4. **Write the review body** in this shape:

   ~~~markdown
   <!-- ai-review: codex 3/9 -->
   ## codex review 3/9 · GPT-6 Astra (medium) · <head sha, 7 characters>

   **Verdict:** No changes required.
   <!-- or: **Verdict:** Changes suggested: 3 (P1 1, P2 1, P3 1) -->

   ### Findings
   1. **P1** `crates/x/src/y.rs:42`: what is wrong, the evidence, the fix. (inline)
   2. **P2** `crates/x/src/z.rs:10`: … (outside the diff; fix below)
      ```diff
      - old
      + new
      ```

   ### Checked
   - One line per area you examined and found sound, so a clean verdict says what it covers.
   ~~~

   The first line is the counting marker, exactly `<!-- ai-review: <reviewer> <n>/9 -->`.
5. **Post it as one review**, event `COMMENT`. The PR's author is the same GitHub account,
   and GitHub refuses approve or request-changes on your own PR. Each finding on a line the
   diff adds or keeps gets an inline comment, with a ```` ```suggestion ```` block when the
   fix is a direct replacement of those lines. `line` is the line number in the new file,
   `side` is `RIGHT`, and a multi-line suggestion adds `start_line` and `start_side`. A
   suggestion replaces exactly lines `start_line` to `line`.

   ```bash
   gh api repos/{owner}/{repo}/pulls/N/reviews --method POST --input - <<'JSON'
   {
     "commit_id": "<headRefOid>",
     "event": "COMMENT",
     "body": "<the review body>",
     "comments": [
       { "path": "crates/x/src/y.rs", "line": 42, "side": "RIGHT",
         "body": "**P1**: what is wrong and why.\n\n```suggestion\n    let total = a.checked_add(b)?;\n```" }
     ]
   }
   JSON
   ```

   Send the payload on stdin as shown. If your shell can't take a here-document, write the
   payload to `target/review/<reviewer>-<n>.json` (gitignored) and pass that path to
   `--input`. With no inline findings, send `"comments": []`.

   GitHub rejects the **whole** review (HTTP 422) if any inline comment points at a line
   outside the diff. On a 422, move every inline finding into the body under **Findings**,
   with its fix as a ```` ```diff ```` block, and post again with `"comments": []`. If that
   fails too, post the body with `gh pr comment N --body-file -`.
6. **Finish** by printing the posted review's URL (`html_url` in the response). If nothing
   could be posted, print the error and the full review body so the requester can post it.

**Never:** edit, create or delete files outside `target/review/`; commit, push, or change
branches; approve, request changes, merge, close, label, or resolve threads; post more than
once per run.

### Answering a review

This part is for the PR's author, and it applies to any human reviewer's comments too.
A review comment is a claim. Check each one against the code. Fix the real ones. Reply to the
rest with the reason: in the inline thread with
`gh api repos/{owner}/{repo}/pulls/N/comments/<comment id>/replies -f body='…'`, or with a
PR comment that quotes the finding. Then push and request the next round.

Stop at the first of these:
- a round brings nothing new;
- the findings have converged to superficial nits: wording, naming, a missing doc line, a test
  that could be tighter, nothing that changes behaviour or lets a wrong body pass a test that
  matters;
- the owner calls a round the last one;
- the reviewer reaches 9/9.

Handle the last round's findings in one pass, without requesting another round: fix the real
ones, and verify with the full check. Then list every finding still open in a PR comment, with
what was decided about each, and promote the PR. Rounds past the point of convergence cost a
review cycle each and find less every time. In genesys-audit-injestor#6, rounds 4 to 6 found
only rare echo cases and doc wording.

## Gotchas

Properties of cargo, clippy, git and the harness that each cost a round trip, a broken build
or a missed bug in the project this scaffold came from. Verify before relying on one; tools
change. Add your own project's below them.

### The loop in a Rust workspace

- **All `_tests.rs` files of a crate compile into one test target.** One test-writer's
  half-written file breaks every sibling's validation run in that crate. Briefs tell
  sub-agents to wait and retry on errors outside their file. Smaller crates make this cheaper.
- **Parallel agents serialise on cargo's build lock** (one `target/`). Correct, just slower.
  Don't run cargo yourself while a phase is out unless you need the answer.
- **Bodies hiding in `impl` blocks are still bodies**: `Display` impls, constructors and trait
  methods each need an `@stub` and an item number. Several stubs in one file belong to one
  item, because a file has one owner per phase.
- **`check-file.sh` skips tests for `types.rs`, `lib.rs` and `mod.rs`**, so an item whose
  contract lives in `types.rs` names its test command explicitly in the brief.
- **Crate-private contracts are dead code until their caller has a body.** For a crate whose
  door exports only the top composite, a temporary crate-level
  `#![allow(dead_code, reason = ...)]` in `lib.rs`, removed when the root is implemented,
  beats per-item allows that the wrong implementor would have to delete.
- **`stub!` can't satisfy an `impl Trait` return** (`!` falls back to `()`). A proptest
  strategy contract returns `BoxedStrategy<T>` instead, without `#[must_use]`, which it
  already carries.
- **A `const` item that calls a stubbed `const fn` fails the build** (E0080), because the stub
  runs at compile time. Prove constness through a `const fn` wrapper until the body lands.
- **To change an implemented function, put `// @stub <n>` above its working body** rather than
  replacing the body. The write guard admits the implementor, the changed tests go red by
  assertion, and callers in other crates stay green meanwhile. Commit tests and body together.
- **Adding an enum variant forces a sweep before contracts compile.** Every `match` gets a
  re-opened `@stub` arm for the new variant only, and irrefutable `let Enum::X(..) =` in tests
  becomes let-else. Grep for both before the first clippy run.
- **`rustfmt <file>` follows the `#[path]` test module**, so an implementor's format check can
  fail on the test-writer's file. `cargo fmt --all` at phase verification settles it (solo
  architects only; under an orchestrator, `rustfmt` your own files).
- **clippy's `allow-*-in-tests` covers `#[test]` functions only.** Helpers in `_tests.rs` or
  `tests/api.rs` still trip `unwrap_used`, `indexing_slicing`, `too_many_arguments`,
  `needless_pass_by_value` and `unnecessary_wraps`. Say so in the brief. Testing-module and
  test-support contracts need the `too_many_lines` allow on `mod tests` too.
- **Drop `shared` from a crate's `[dependencies]` once its last stub is gone**, or
  cargo-machete fails the check. `git grep` misses untracked files, so check with `grep -rn`.
- **Binaries use `std::env::args_os()`, never `args()`** (which panics on non-Unicode), and
  never discard a write to stdout: a failed report must fail the exit code.
- **Tools run from inside `cargo xtask` inherit `cargo run`'s environment** (`CARGO_PKG_NAME`
  and friends). Suspect the environment first when a tool behaves differently under the
  runner than in a shell.
- **Dispatch an item's I brief as soon as its T report is red.** Waiting for the whole
  phase's T reports gains nothing, since files don't overlap.
- **Put the no-copies rule in the very first T briefs.** Test-writers working in parallel each
  declare the same builders and strategies otherwise, and undoing it costs whole phases.
- **Scan for duplicates before the final pass**: hash every non-`#[test]` `fn` body and
  `const` value across `_tests.rs` and `tests/api.rs`, and also list helper names found in
  more than one file. Hashing alone misses copies that differ only in what they call.
- **Test-writers stop at about 40 turns.** Tell them to finish each file before starting the
  next.
- **`cargo nextest` filters match by substring:** `types::tests::` also runs
  `ring::types::tests`. Give expected counts by module path in briefs.
- **Count a contribution's new tests per crate** (`cargo nextest run -p <crate>`), never by
  subtracting workspace totals.
- **Deliberate breaks:** never run one while any agent in the phase is still validating; it
  will see the transient edit. Pick a break that removes something, not one that reorders.
  Search for the target text from the first `fn` line, since doc comments quote the same text.
  Script them in a scratch copy, restore in a `finally`, time each out on its own (nextest has
  no default timeout), and check that each actually broke something.
- **A monotonicity property doesn't catch a body that never takes the last step.** Pair it
  with table tests naming each transition.
- **Measure the edge cases on purpose** (empty input, ties, truncation). Never let a brief
  assert a value nobody has measured.
- **Numeric code that must be chunking-invariant never passes a position as one `f64`.**
  `whole + fraction` rounds differently as `whole` grows; pass the parts separately.

### Under an orchestrator (for architects)

- **Start test-writers and implementors in the foreground** (`run_in_background: false`). A
  background sub-agent's completion notice can go to the orchestrator instead of you. Several
  Agent calls in one message still run in parallel.
- **Foreground `sleep` is blocked in Bash.** To wait on a condition, loop:
  `for i in $(seq 40); do <check> && break; sleep 15; done`.
- **A crate directory with a `Cargo.toml` and no `src/lib.rs` breaks every cargo command in
  the checkout**, siblings' included. Write the manifest and `lib.rs` in the same step.
- **Shared crates must compile at every moment.** Manifest before the stub, a type and its
  users in one step, one implementor at a time in the vocabulary crate. A half-written shared
  crate breaks every sibling's `--tests` build.
- **Use `cargo clippy --no-deps`** for scoped checks: clippy otherwise lints every workspace
  crate you depend on, including a sibling's mid-write.
- **When a sibling's half-written crate blocks your build,** check in a `git archive HEAD`
  copy under a prefixed scratch directory with its own fresh `CARGO_TARGET_DIR` (an archive
  has old timestamps, so a reused target can build stale code).
- **`git commit -- <dir>` commits only tracked files.** `git add` each phase's new files, and
  check `git status --short -- <territory>` before reporting. A deleted file can't be `git
  add`ed; name it in `git commit -m .. -- <paths>`. A missing `mod x;` line hides a contract
  and its tests without any error, so check every `src/*.rs` is mounted.
- **Keep implementors' over-limit drafts in scratch.** A draft over 40 lines left in the
  checkout fails every sibling's clippy with `too_many_lines`.
- **Removing or reshaping a type other workstreams build:** prepare the change off the
  checkout, and apply it on the orchestrator's "NOW", or add the new path first, move the
  callers, and remove the old one last.
- **The scratchpad is shared by every agent in the session.** Prefix every scratch path with
  the workstream name, put the prefix in every brief, and never write to `/tmp`.
- **Never kill processes you didn't start.** A machine-wide `cargo` kill takes siblings'
  builds down with it.

### Orchestrating

- **Only the main session's `Agent(...)` list is enforced**, and it caps the whole tree. An
  edit to it takes effect after a restart. Probe-spawn a `test-writer` before the first wave.
- **A stopped agent can't be resumed**, but a rate-limit (429) stop can: `SendMessage` resumes
  every architect under its old id. Brief each with its last commit, uncommitted files and last
  words, and check first for deliberate breaks left on disk.
- **Back up `.claude/territories/` to the scratchpad after every change.** It has vanished
  twice mid-effort, with no delete in any log.
- **Write a territory file with the Write tool only after reading it** (Write refuses an
  unread existing file), or with a one-line `printf` in Bash.
- **Agents' commit hashes are wrong about one time in five.** Check `git log` before relaying
  one.
- **An architect can report DONE while its sub-agents still run.** Correct it at once, since
  DONE triggers hand-overs.
- **The workstream that holds the vocabulary crate becomes the CROSS hub.** Spawn it first,
  keep it alive until the other contracts settle.
- **"No copies" breeds shared crates mid-loop.** Give each new shared crate to its first
  user's territory, add the workspace line at once, and have the old copies deleted by whoever
  holds them. Record each in the plan.
- **`Cargo.lock` traffic is constant.** Commit what's on disk after checking each changed line
  belongs to a known manifest change, and run `cargo deny check` when new packages appear.
- **Don't re-run CI on `main` after a squash-merge whose PR run just passed.**
- **When usage limits run out mid-effort,** halt cleanly alone: stash everything (`-u`) and pin
  the stash to a `wip/` branch, check the workspace at HEAD, and write where each workstream
  stopped into the plan. Ask architects for small commits often, so a stop loses little.

### Windows

- **The Bash tool collapses `\\` to `\` before bash sees the command.** Anything whose
  backslashes matter (JSON with Windows paths, regexes, `\n` in Rust strings) goes through the
  Write tool or a script file.
- **`.gitattributes` forces LF.** Without it the bash hooks check out as CRLF and break.
  Python's `Path.write_text` writes CRLF on Windows too; scripted edits use
  `read_bytes`/`write_bytes`.
- **`gh` inside a Codex sandbox gets HTTP 401**: the sandbox can't reach the keyring. The
  review command above passes `GH_TOKEN` for that reason.
- **The checkout may be shared with the owner's other sessions.** Run `git status` and
  `git branch --show-current` before committing or pushing, and stage only the paths the
  contribution owns: `git add -A` sweeps in the owner's own uncommitted edits.
- **Memory edits dirty the tree, and a reviewer refuses a dirty checkout.** Write memory notes
  after the review loop ends. Notes that ride on the next branch get a line in its PR body's
  **Decisions to veto**.
- **An `.exe` freshly linked can be locked for a moment** (LNK1104). Rerun the check before
  debugging it.
