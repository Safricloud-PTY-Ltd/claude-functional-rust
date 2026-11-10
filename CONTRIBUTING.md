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

## Delegating a brief to Codex

Applies when `CLAUDE.md`'s **Brief runner** is `codex`. Each test-writer (`T-…`) and
implementor (`I-…`) brief goes to Codex instead of a Claude sub-agent. Codex reads
`AGENTS.md` by itself, which sends it to `CLAUDE.md`, the code-standards skill and its role
file. No Claude Code hook runs for Codex: `run-brief.sh` takes the brief's file locks and
lists what the run touched, `AGENTS.md` states the rest of the rules, and you verify.

### 1. Write the brief

Use the architect's brief format, in `<scratch>/codex/<id>.brief` (e.g. `T-app-5.3.brief`),
written with the Write tool. `<scratch>` is your scratch folder: under an orchestrator, the
`scratch` line of your territory file, the only place outside the repo the write guard lets
you write; as the main session, your own folder in the session scratchpad. It needs a `Write:` line of bare repo-relative paths (or `none`):
the test file for a T brief; for an I brief every file carrying a stub tagged with its item.
Prose goes on the next line; any word without a `/` exits 64. The script adds the role
header. Every T brief says "import builders and strategies from test-support; one another
test file would want is a Clarify, never a copy".

### 2. Run it

```bash
.claude/codex/run-brief.sh <scratch>/codex/<id>.brief            # the role's effort
.claude/codex/run-brief.sh <scratch>/codex/<id>.brief high       # a hard body, or a property-heavy test file
```

From the repo root, with `run_in_background: true` (never a shell `&`), one Bash call per
brief, at most three at once. Never chain a run with anything else in one command: a compound
Codex command asks the owner for approval even in bypass-permissions mode. Codex gets
`workspace-write` with your PATH and no network. Model, effort and speed come from the role's
settings (`CODEX_TEST_WRITER_*`, `CODEX_IMPLEMENTOR_*`: **Models** in `CLAUDE.md`); the
optional effort argument overrides the effort for one brief. A model outside
`CODEX_ALLOWED_MODELS`, or a speed other than `base`, `fast` (service tier `priority`) or
`ultrafast`, exits 64 before anything runs. Beside the brief the script writes `.prompt`, `.out`
(full log), `.report` (last message), `.touched`, `.stray` and, last, `.exit` (plus its own
`.allowed`, `.before` and `.after`); its header lists the exit codes.

Wait on your own runs with `wait-briefs.sh`, not on the background-task notice, in a Bash
call with `timeout: 600000`:

```bash
.claude/codex/wait-briefs.sh <scratch>/codex/T-app-5.3.brief <scratch>/codex/T-app-5.4.brief
```

It prints one line per brief and exits 0 when every run has written its `.exit`, 3 when its
540-second limit came first (call it again), and 4 when a run died: no `.exit`, and the
process in its `.pid` is gone, so its background task was killed. A dead run's touched list
was never written and its `codex-<id>` locks are still held: read its `.out` and
`git status` before anything else. A mistyped brief path writes no `.exit` and no `.pid`, so
it shows as dead after 30 seconds; read the run's own output. Delete a brief's old `.exit` in
the foreground before relaunching it. Never end your turn while a run is out. Keep commits
and dispatches in separate Bash calls.

### 3. Check what it touched

- **Locks.** The script exits 3, before Codex starts, if another agent holds a `Write:` path,
  then locks them as `codex-<id>`. Release them once you've verified the run, with
  `.claude/scripts/release-locks.sh codex-<id>`, and before handing the brief to a Claude
  sub-agent.
- **Strays.** `.touched` lists every non-ignored path whose content changed during the run;
  `.stray` those not on `Write:` (exit 4; a non-zero Codex exit is 70 and wins over it, with
  both lists still written). The script's output labels each stray with its likeliest owner:
  `locked by <owner>`, `in <workstream>'s territory` (a sibling's agent, almost always),
  `in <yours>'s own territory` (Codex, or one of your other briefs), or `RESERVE: in no
  territory`. Siblings edit the same checkout, so check the locks, the other briefs'
  `Write:` lines and `git diff` before calling a stray Codex's. Undo Codex's strays in your
  territory with Edit; report others to the orchestrator.
- **Each run starts cold.** A permission you gave one run is unknown to the next, so put
  every permission a brief grants in the brief itself, every time ("you may add the imports
  the body needs" most often).
- **Put outside facts in the brief.** The brief's own text reaches Codex in its prompt, but a
  file outside the repo may not: in the sister TypeScript scaffold, Codex's sandbox couldn't
  read the scratchpad or other repositories. Copy any outside fact a brief needs into the
  brief itself, verbatim.

### 4. Verify and resume

- Read `.report` (or the end of `.out`), then verify as for any sub-agent, plus
  `rustfmt --check --edition 2024` on every file it wrote.
- Answer a NEEDS, CLARIFY or BLOCKED by resuming the same session. Widen `Write:` first if
  the answer adds files:

  ```bash
  .claude/codex/run-brief.sh --resume <scratch>/codex/<id>.brief <scratch>/codex/<id>.answer-1
  ```

- If the patch tool stalls ("patch is pending"), resume with "write whole files".

### Limits

- Three runs at once per architect, never two on one file. All architects share one ChatGPT
  limit and one cargo build lock.
- Group two to four related contracts per brief.
- Two failed runs (unverifiable report, stray edit, no report) hand the brief to a Claude
  sub-agent; record it under **Deviations**. Out of usage: switch the rest of the phase.
- Run `cargo fetch` before a phase whose manifests changed; the sandbox can't download.
- A failure that needs a device, socket or certificate store may be the sandbox: re-run it
  outside before calling it real. A crate that needs a device checks the sandbox can open it before its first brief.

## Pull request review

The **PR reviewer** setting in `CLAUDE.md` decides whether this section applies. With `none`,
skip it: a PR goes from draft to ready to merge. With `codex`, `claude` or `both`, pull
requests are reviewed by that model, or each of them, on request, never automatically.

| Reviewer | Model | How it starts |
|---|---|---|
| `codex` | `CODEX_REVIEW_MODEL`, `_EFFORT`, `_SPEED` (**Models** in `CLAUDE.md`), via the Codex CLI | By request, as below. |
| `claude` | `CLAUDE_REVIEW_MODEL`, `_EFFORT`, via Claude Code headless | By request, as below; also the fallback when Codex can't run. |

The values live in `.claude/settings.json` `env`, and `request-review.sh` reads them. The CLI
flags it passes were measured on 2026-09-22; update the script when they change.

**A PR is opened as a draft and stays one until its review loop is complete.** CI skips
drafts. Marking the PR ready (`gh pr ready <n>`) starts CI, so it runs once per PR and not
once per review round.

Requested reviews run on the owner's own subscriptions (ChatGPT for Codex, Claude for Claude
Code), not on API keys. Each reviewer posts its own review to the PR with the `gh` CLI.

### Requesting a review

Request one when the PR is opened, and again after every push that changes it. Each reviewer
reviews a PR **at most 9 times**. Every review's first line is a marker,
`<!-- ai-review: <reviewer> <n>/9 -->`, and the round is the number of markers already on the
PR plus one. Request a round with the script, one Bash call per reviewer:

```bash
.claude/scripts/request-review.sh <pr number> <codex|claude>
```

It takes minutes, so run it with `run_in_background: true`. With PR reviewer `both`, start
both reviewers at once, one call each, and wait for both. The script:

- refuses a Codex model outside `CODEX_ALLOWED_MODELS` or an unknown speed, before anything
  else (exit 64);
- counts the reviewer's markers and stops at 9 (exit 3);
- checks that the checkout it lives in is exactly the PR's head with a clean tree, since the
  reviewer reads that checkout: commit everything and push first;
- deletes a failed earlier attempt's payload, `target/review/pr<N>-<reviewer>-<n>.json`, and
  tells the reviewer to write its payload there;
- runs the reviewer and, when it exits, confirms that the round's marker landed. A run that
  exits without posting fails the script (exit 1) and doesn't count against the 9: read its
  output, fix the cause, and request again.

While agents are still working in the checkout, review from a worktree at the PR head
instead: `git worktree add --detach <scratchpad>/review-wt <head>`, then run the worktree's
own copy of the script (`<scratchpad>/review-wt/.claude/scripts/request-review.sh`) with its
own `CARGO_TARGET_DIR`, then `rm -rf` the worktree and `git worktree prune`.
(`git worktree remove` fails on Windows' path-length limit once `target/` exists.)

**Codex.** Inside Codex's sandbox, `gh` can't reach the system keyring, so the script passes
`gh auth token` in as `GH_TOKEN` for that one run. `workspace-write` with network on is the
least access under which posting works.

**Claude.** `--setting-sources user` keeps this repo's `agent: orchestrator` default and its hooks
out of the reviewer's session. `dontAsk` refuses anything not on the allow-list, so the
reviewer can read, run a crate's tests, and talk to GitHub, but can't edit files outside
`target/review/` or run git commands that change state. `Edit(target/review/**)` is what
admits the Write tool there (a `Write(...)` rule is not matched), and the review payload goes
through a file in it: a here-document on `gh api --input -` is refused under `dontAsk`.

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

   Write the payload with your file tool to the payload file your request names
   (`target/review/pr<N>-<reviewer>-<n>.json`, gitignored), then post that file:

   ```json
   {
     "commit_id": "<headRefOid>",
     "event": "COMMENT",
     "body": "<the review body>",
     "comments": [
       { "path": "crates/x/src/y.rs", "line": 42, "side": "RIGHT",
         "body": "**P1**: what is wrong and why.\n\n```suggestion\n    let total = a.checked_add(b)?;\n```" }
     ]
   }
   ```

   ```bash
   gh api repos/{owner}/{repo}/pulls/N/reviews --method POST --input target/review/pr<N>-<reviewer>-<n>.json
   ```

   Never send the payload on stdin or through a here-document: posts sent that way have been
   lost, and the claude reviewer's shell refuses a here-document. With no inline findings,
   send `"comments": []`.

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
`gh api repos/{owner}/{repo}/pulls/N/comments/<comment id>/replies -F body=@<file>` (the reply
written to a scratch file first, so quotes and backticks survive), or with a PR comment that
quotes the finding. Reply to every thread before the next round, which reads the replies.
List threads with `gh api --paginate`: GitHub returns 30 at a time. Then push and request
the next round.

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

## The project board

Applies when `CLAUDE.md`'s **Project** setting is `yes`. The repository then has one GitHub
Project, and the main session keeps it: the orchestrator, or an architect running solo.
Their agent files say when each step happens. This section says what sits on the board
and how each step is done. With the setting at `no`, or still showing both options,
nothing here runs; issues are still created and labelled.

| The workflow | On GitHub |
| :-- | :-- |
| effort | an issue labelled `effort`; the PR closes it with `Fixes #<n>` |
| workstream | a sub-issue of the effort issue, labelled `workstream` |
| task | a sub-issue of a workstream: a backlog issue the effort adopted, or one it deferred |
| future work | an issue with Status Todo and no parent |
| waiting on a person | `blocked-on-product-owner` or `blocked-on-dev-owner`, on any of the above |

An effort with one workstream has no workstream issues: its tasks sit under the effort
issue. Plan items are not issues. They live in `plan.md`; one issue per contract would be
hundreds of short-lived ones.

Status is the project's own field with its default options, Todo, In Progress and Done. A
new project's built-in workflows do two of the moves: an item lands at Todo when it is
added, and goes to Done when its issue closes or its PR merges (within about five seconds,
measured 2026-11-10). So you set In Progress, and closing an issue is how it finishes. A
parent issue does not close when its last sub-issue does, and its Status stays where it
was: the effort issue finishes when the PR's `Fixes` closes it.

**No step rests on memory.** The numbers live in the plan's **Board** section, and at the
top of the review until the plan exists: the
project's number and owner, the effort issue, each workstream issue, and each adopted task
with the issue it was put under. Every step names the issues it acts on from there. A
session restarted mid-effort looks the project's ids up again; it can't look up which
tasks this effort adopted, or which parent it gave each.

### The commands

`<o>/<r>` is the repository (`gh repo view --json nameWithOwner`). Write every id
literally into the next command, one lookup and one use, so each call shows what it acted
on. Text with quotes or newlines (an issue body) goes in a scratch file and `--body-file`.

- **Find the board.** It is the open project linked to this repository:
  `gh api graphql -f query='query { repository(owner: "<o>", name: "<r>") { projectsV2(first: 20) { nodes { number title url closed id } } } }'`.
  With none, create one named after the repository and link it:
  `gh project create --owner <o> --title '<r>' --format json`, then
  `gh project link <number> --owner <o> --repo <o>/<r>`. A project shows in that query only
  after it is linked. With more than one open, stop and ask the owner which, and write the
  number into the plan's **Board** section so no later step asks again. The choice lasts
  the effort; the owner makes it last by unlinking the others (`gh project unlink`). Never
  match a project by its title: titles aren't unique in an organisation.
- **Its ids, once per session.** `gh project field-list <number> --owner <o> --format json`
  gives the Status field (the entry of `.fields` named `Status`), its `id`, and each
  option's `name` and `id` under its `.options`. The project's own `id` is in the query
  above.
- **Put an issue or a PR on it.**
  `gh project item-add <number> --owner <o> --url <its URL> --format json` prints the item,
  whose id is its `.id`. It lands at Todo by itself. An issue adopted from the board is on
  it already: adding it again is expected to return the same item (not run). If it errors,
  take the item's id from a read of the board.
- **Read the board.**
  `gh project item-list <number> --owner <o> --limit 1000 --format json` shows each item's
  `.status`. Never leave `--limit` off: the default prints the first 30 items only, and the
  item you are checking may not be among them (a 75-item board printed 30, measured
  2026-11-10). If `.totalCount` is above the number of items printed, raise the limit.
  Every read of the board uses this command.
- **Set an item's status.**
  `gh project item-edit --id <item id> --project-id <project id> --field-id <Status field id> --single-select-option-id <option id>`.
- **Finish an issue.** `gh issue close <n>`. The board moves a closed item to Done itself.
  Confirm it did with a read of the board, and set Done by hand if the project's workflow
  has been turned off.
- **Make #c a sub-issue of #p.** `gh api repos/<o>/<r>/issues/<c> -q .id`, then
  `gh api -X POST repos/<o>/<r>/issues/<p>/sub_issues -F sub_issue_id=<that id>`. The id is
  the issue's own, not its number, and the response is the parent issue. Read the children
  back with `gh api repos/<o>/<r>/issues/<p>/sub_issues -q '.[].number'`; `gh issue view
  --json` has no sub-issue fields. GitHub refuses an issue that already has a parent (HTTP
  422, "Sub issue may only have one parent"). Leave it where it is and say so in the
  review: moving it (`-F replace_parent=true`) rearranges something the owner arranged, so
  it needs the owner's yes. One exception: when the parent is a closed issue labelled
  `effort` or `workstream`, the task was deferred by an earlier effort and that tree is the
  agents' own. Move it with `-F replace_parent=true` and note the old parent in the plan.
- **Take #c out from under #p.** Only when
  `gh api repos/<o>/<r>/issues/<p>/sub_issues -q '.[].number'` lists it, and `<p>` is the
  parent the plan says this effort gave it. Then
  `gh api -X DELETE repos/<o>/<r>/issues/<p>/sub_issue -F sub_issue_id=<the id of #c>`: the
  path ends `sub_issue`, singular. A task the effort could never attach, because it had a
  parent already, isn't listed: it stays where the owner put it, and only its Status
  changes.
- **Close as not planned.** `gh issue close <n> --reason "not planned"`, for an abandoned
  effort's own issues, with the owner's yes.
- **What a PR will close.** `gh pr view <n> --json closingIssuesReferences`.
- **A label that doesn't exist yet.** `gh label create <name>` first, for `effort`,
  `workstream` and the two `blocked-on-` labels: a missing label fails `gh issue create`.

A token without the `project` scope fails the first of these. Only the owner can fix that,
with `gh auth refresh -s project`: say so and wait, because no board step can run without
it. Draft items are never used: a draft can't carry a label, a parent or a `Fixes`, and
`gh issue list` doesn't show it. `gh project delete` doesn't prompt.

### The steps, from the first to the last

1. **Effort issue.** Find the board. Open the issue with the label `effort`, a title that
   starts with the effort's id (`2026-10-01-invoicing: invoices by email`), and the ask and
   its source as its body, or label the existing issue when the ask is one issue. Put it on
   the board and set it In Progress. The id in the title is how a restarted session tells
   this effort's issue from one an earlier effort left open.
2. **Orient.** Read the board. The review names the Todo items to adopt and every item
   still In Progress other than the effort issue.
3. **Workstreams and adoptions**, after the plan. Each workstream issue carries the label
   `workstream` and a body with its territory, wave and plan path, and is a sub-issue of
   the effort issue, on the board at Todo. Each adopted issue goes on the board and under
   its workstream issue, or under the effort issue when there are no workstream issues.
4. **In Progress.** A workstream issue and the tasks adopted under it, when its architect
   starts. An adopted task left at Todo would sit beside future work until it closed.
5. **Workstream done.** Close its issue when the work is verified. Its adopted tasks wait
   for the PR.
6. **Out of scope.** With the owner's yes only: the task back to Todo, and out from under
   the parent this effort gave it. Agents defer nothing of their own.
7. **Deferred.** A `blocked-on-` issue, on the board at Todo, under the issue it came from.
8. **The PR.** Its body ends with one `Fixes` per issue, each with its own keyword
   (`Fixes #12, fixes #14`), for the effort issue and every adopted task completed. Put the
   PR on the board and set it In Progress. Read what it will close, when it opens and
   after every edit of the body, and give the owner the numbers.
9. **Before the merge.** Read what it will close once more. The effort issue and every
   task still adopted must be listed. Fix the body before merging if one isn't.
10. **After the merge.** Confirm each closed and shows Done, setting Done by hand where an
    issue closed but its item didn't move. Close any sub-issue of the effort issue labelled
    `workstream` that is still open. Report by number any adopted task still open: that is
    a missed `Fixes`. Other sub-issues still open (list them under the effort issue, then
    under each workstream issue, because tasks sit one level down) are deferred tasks, and
    they stay open on the board.

### What the board doesn't do

- **A merge by hand runs no step 10.** The board still moves closed issues to Done by
  itself, and step 8 has already told the owner what the PR will close. A `Fixes` lost
  after that, by a body edited by hand, shows at the next effort's orient, as a task still
  In Progress.
- **An abandoned effort stays as it was** until the owner says to tidy it: adopted tasks
  back to Todo and out from under their parents, the effort issue and any open workstream
  issue closed as not planned.
- **Turning the setting on backfills nothing.** Efforts are tracked from then on. An open
  issue joins the board when an effort adopts it or the owner adds it.
- **A workstream issue closes before the merge**, when its work is verified. An abandoned
  PR leaves the board saying Done for work that never landed.

### What has and hasn't been run

The commands were run once, in order, in the sister TypeScript kit against a throwaway
project in an organisation with gh 2.89 (2026-11-10). In this kit, the find-the-board query
was run once, the same day, against a repository with no linked project. Never run in
either kit: a whole effort with the setting at `yes`; taking an issue out from under its
parent; moving one with `replace_parent`; closing an issue as not planned, and whether the
board then shows it Done; adding an item that is already on the board; a pull request as
an item; a repository owned by a user, not an organisation; an organisation that forbids
its members to create projects; and a `Fixes` closing the effort issue at a merge. The
first time one of those happens, write down what GitHub printed, here.

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
- **A slow or perf test runs only under its own nextest profile.** A profile's
  `default-filter` bounds a name filter and `-E` as well, so
  `cargo nextest run -p <crate> <fn>::slow_tests::` with no `--profile slow` collects
  nothing and exits 4, "no tests to run". A misspelt profile exits 96 and lists the known
  ones. Plain `cargo test` has no profiles and runs every suite.
- **nextest refuses a `binary()` matcher that matches no binary**, and it compiles every
  profile's filter on every run. A filter on `binary(/_slow$/)` alone fails every run in a
  workspace that has no such test file yet (exit 96). That is why the profiles in
  `.config/nextest.toml` also name `xtask` and then exclude it. A probe that plants every
  case never meets the empty one: measure a filter in a fresh workspace too.
- **nextest doesn't show a passing test's output.** To read a value out of a test run, use
  `--no-capture` or a deliberately failing assertion.
- **Mutation testing sees only the suite it runs.** `.cargo/mutants.toml` keeps
  `cargo mutants` on nextest's default profile, the normal suite, so a function only a slow
  test reaches comes back with its mutants missed. Give that logic a normal test where the
  contract allows one (a fake clock, a smaller input).

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

### Planning and review

From two efforts in the sister TypeScript kit on 2026-11-10, seven review rounds between
them, and the port of both into this kit.

- **Walk a new flow end to end before the first review.** One PR added a flow of several
  steps in prose. After round 1, every finding in four more rounds was a case nobody had
  walked: a merge by hand, an abandoned effort, a second effort on the same board, a
  restarted agent, an effort with no architect. One hostile walk through the whole
  lifecycle before opening the PR would have found most of them.
- **No step rests on what an agent remembers.** An agent that can't be resumed is started
  again with nothing, and a main session can be restarted or compacted. Three rounds of
  findings were instances of this one rule. Every step names what it acts on, and the
  numbers live in the plan.
- **Check before the irreversible step, not after.** A missing `Fixes` reported after the
  merge can't be repaired by editing the PR body. Reading what the PR will close before
  merging catches it while it can be.
- **An owner's answer about where something runs is a ceiling.** The owner chose "CI only"
  for a test suite. A draft then added one local run for convenience and listed it as
  something he could veto. It contradicted the decision and was removed. An extra belongs
  in **Decisions to veto** only when the owner hasn't spoken on the point.
- **Don't invent exceptions to an owner's rule.** "For any UI ask" gained an unrequested
  "unless a design already exists". A reviewer had it removed.
- **Reviewer findings are claims, and so are replies, and so is a sub-agent's NEEDS.**
  Measure each before you fix it. Every finding in those seven rounds was real, and two
  were stronger than stated: a wrong word about mutation testing hid a failing run, and a
  suggested check after the merge became a check before it.
- **Ask the shaping questions before designing details.** One free-text answer from the
  owner replaced a whole agent with a section in an existing one, and everything designed
  around the agent was thrown away.

### Probing and measuring

- **A default cap hides in every list command.** `gh project item-list` prints 30 items,
  and `gh api` lists return 30 a page. A probe with three items never shows it. Probe a
  list command against something large.
- **Probe a new agent by spawning it, in a new session.** The agent types a session can
  start are fixed when it starts: a designer file written mid-session was refused there as
  "Agent type 'designer' not found" (this kit, 2026-11-10), although the TypeScript kit once
  saw a new type appear in the running session. And the `tools:` line doesn't decide what
  an agent gets: in that kit a designer's real tool list differed from its line in three
  places. Ask a spawned agent for its tool list before you trust the line.
- **Know what an upgrade carries.** `npx … --force` overwrites `.claude/`, `CLAUDE.md`,
  `CONTRIBUTING.md` and `AGENTS.md`. It never touches the files the scaffold installed at
  the root: `Cargo.toml`, `.config/nextest.toml`, `.cargo/`, `xtask/`, CI, the pre-push
  hook. A change to those says what an installed project copies by hand (**Upgrading an
  installed project** in the `project-scaffold` skill).
- **Measure a claim from another repository before porting it.** A config snippet in a
  source issue predated the kit's layout and would have dropped two things. In the port to
  this kit, a filter measured only with every kind of test planted failed in a fresh
  workspace.
- **Say which machine showed what.** "Measured in a scratch project on Windows, never in
  CI" is a different claim from "works".

### Changing the harness

- **The hooks have tests**, run with node's own runner, from any shell:
  `node --test ".claude/hooks/*.test.cjs"`. Each test builds a throwaway project and runs
  the hook on hand-built input. Run them after any change to a hook, and add a case for
  the change, red first. On Windows the helper finds Git Bash through git, because from
  PowerShell a bare `bash` can be WSL's. They take a minute or two there: every case starts
  a bash.
- **A hook test stubs every tool the hook starts or probes**, or it runs the machine's own.
  The per-edit hook runs `rustfmt`, `cargo clippy`, `cargo nextest --version`, and
  `cargo nextest run` or `cargo test`. Assert on what the stub recorded, so a test that
  reached the real tool fails.
- **Hook input in a test has the shape Claude Code sends**: `file_path` absolute, in the
  platform's spelling, backslashes on Windows. Build it with `path.join`.
- **A `..` path case built with `path.join` tests nothing**: the join resolves it before
  the hook sees it. Join the segments with `path.sep` yourself.
- **Cover a file that exists and one that doesn't.** They take different branches of a
  real-path lookup.
- **A new agent type the write guard doesn't know writes anywhere** Claude Code's own
  permissions allow. An agent that should be fenced gets its own arm in `guard-writes.sh`,
  with a test, as the designer has.
- **Never put a placeholder in a replace script's replacement.** A "test" replacement
  matched and wrote the literal placeholder into an agent file. Use the Edit tool for a
  block, and search for placeholders before every commit.

### Windows

- **The Bash tool collapses `\\` to `\` before bash sees the command.** Anything whose
  backslashes matter (JSON with Windows paths, regexes, `\n` in Rust strings) goes through the
  Write tool or a script file.
- **A directory may be spelled two ways**: with a short name (`C:\Users\SOMEON~1\…`, which
  is how some machines spell their temp directory) and with the long one. A hook that
  compares paths needs real paths on both sides; a prefix strip matches nothing. The
  per-edit hook once hid every clippy error from sub-agents that way. `territory.cjs`'s
  `repoRelative` is the one place that resolves a path, and the temp directory's own
  spelling makes a useful test input.
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
