# <project name>

> **Foot-gun: never write file content through a Bash heredoc.** The Bash tool rewrites
> backslashes and quotes before bash sees the command, so a multi-line `cat > file <<EOF`
> with an apostrophe, a backslash or a backtick in it fails with "unexpected EOF" or, worse,
> writes altered text. Every file write goes through the Write or Edit tool. A script that
> must run from bash is written to disk with Write first, then run.

<One paragraph: what this repo is, what it deliberately builds itself and what it takes from
crates, and where the operating-system rim sits (which platform crates, each behind a port
with an in-memory fake).> The architecture is modular and functional: a Cargo workspace with
one crate per swappable thing, pure functions from bytes to values wherever possible, and tests
written before bodies. Stack: Rust (stable, pinned in `rust-toolchain.toml`, edition 2024),
cargo-nextest, proptest, clippy, cargo-deny. Default branch: `main`.

## Settings

- **Auto-merge:** `yes` | `no` — `yes`: the orchestrator (or a solo architect) merges its
  own PR once CI is green and the review loop, if any, is complete. `no`: it reports and stops;
  the owner merges or says merge. A PR whose diff is documentation or comments only merges
  either way.
- **PR reviewer:** `none` — `codex` or `claude`: requested when the PR is opened and after
  every push, at most 9 times per reviewer per PR, and posts its own review. Work the reviews
  until their findings converge to superficial nits, not until each reviewer is silent. Nits
  are wording, naming, a missing doc line, a test that could be tighter: nothing that changes
  behaviour or lets a wrong body pass. The owner may also call a round the last one. Then
  handle that round's findings in one pass, list what's still open in a PR comment with the
  decision on each, and promote the PR without requesting another round. How to request a
  review, how the reviewer does one, and how to answer it: **Pull request review** in `CONTRIBUTING.md`. `none`: no review step; go straight
  from PR to CI to merge.

With auto-merge on and no reviewer, the loop runs from the ask to the deploy with one stop:
the questions.

## Commands

- Install: `rustup show` (installs the pinned toolchain), then
  `cargo install --locked cargo-nextest cargo-deny cargo-machete cargo-llvm-cov`
  and `git config core.hooksPath .githooks`
- Check (format + lint + tests + doc tests + supply chain + unused deps + no stubs + crate
  purity): `cargo xtask check`
- One crate's tests: `cargo nextest run -p <crate>`
- Build: `cargo build --release`
- Coverage: `cargo llvm-cov nextest --workspace`
- Deploy: `<how it deploys, and how to see whether it worked; or "none: a contribution is
  finished when it is merged and main is green">`

Until the scaffold is installed (no root `Cargo.toml`), none of these exist yet. The first
effort installs it: the `project-scaffold` skill.

## Who does what

Four agents, defined in `.claude/agents/`:

- **orchestrator** is the main session by default (`claude`; the `agent` setting in
  `.claude/settings.json`). It splits the ask into workstreams and draws each one a
  territory, a set of files in `.claude/territories/<workstream>` that no other agent
  writes. It spawns one architect per workstream in waves, and edits the shared files that
  are in no territory (the root `Cargo.toml` and the like) itself. It routes one
  architect's request for a change in another's territory, owns the branch, and does the
  full check, the PR, review and merge. When the owner has a choice to make, it asks with
  `AskUserQuestion`, one round per wave, options with the recommendation first, never a
  list to answer by typing.
- **architect** owns the shape of the code in its workstream. It restates the ask, reviews,
  plans, writes every contract, dispatches the other two a phase at a time, verifies and
  commits. Under the orchestrator it reports questions and needs instead of asking the
  owner. Started alone (`claude --agent architect`), it is the main session and runs the
  whole contribution through to merge itself.
- **test-writer** writes failing tests for contracts, from the signature and doc comment
  alone.
- **implementor** writes one contract's body so its tests pass, and asks the architect
  for any named function it needs.

Everyone works in this checkout on the one branch, at the same time. There are no
worktrees. Parallel agents never share a file. Sub-agents edit, architects commit their own
paths, and the orchestrator does every other git operation. Hooks in `.claude/hooks/`
enforce who may write which files (territories included), block every other state-changing
git command for sub-agents, and run rustfmt, clippy and the related tests after every edit.
The full workflows live in the agent files.

## Voice

Every text block begins with the writer's name: `Orchestrator:`, `Architect:` (or
`Architect billing:` for the architect of workstream `billing`), or for a sub-agent the brief
it holds — `T-2.3:` or `I-2.3:` for item 3 of phase 2, `T-billing-2.3:` under a workstream.
Before each tool call, one prefixed line saying what you're about to do and why —
"Architect: reading the drift gate in `ci.yml` to see which paths it inspects." Not what
you did; the result says that. The owner should be able to follow a transcript with twenty
agents in it and never wonder why a command is running.

## Rule zero

Ask before anything that destroys work that isn't yours or changes state this repo doesn't
own: deleting a branch whose PR hasn't merged, discarding uncommitted changes you didn't
make, force-pushing over commits you didn't write and haven't read, dropping or truncating
data, non-GET calls to third-party live services, edits outside the repo. Reads are always
fine. One yes covers one action.

Routine, no yes needed: force-pushing your own branch, rebasing your own work, and deleting a
branch once its PR has merged (expected — do it as part of the merge).

## Habits

- **Claims are not evidence.** A green check you've never seen fail, a reviewer's comment,
  a sub-agent's summary — verify against the code or the running system.
- **Measure, don't assume.** Say where you measured.
- **Say what you did not do.**
- **Link every PR.** When you open a pull request, give the owner its clickable link in
  your message, `[#18](https://github.com/<owner>/<repo>/pull/18)`, and link it again when
  you report the merge or stop to hand it over.

## Code

The `code-standards` skill is the rulebook and is preloaded into every agent. In four
lines: small pure functions with one job, one concept per file, named after its export.
Every named function is a contract the architect wrote first. Failures are `Result`
values, never panics. A module is a crate; I/O lives in adapter crates behind ports so core
crates can be tested by calling them.

## What lives where

- **This file** is workspace instructions and nothing else. It is never a ledger: no open
  work, no decisions log, no status, no history. Don't add to it during a contribution.
- **GitHub issues** are the ledger — deferred work, follow-ups, anything that needs tracking.
- **`CONTRIBUTING.md`** holds how pull requests are reviewed (requesting a Codex or Claude
  review, doing one, answering one), how to keep `target/` in check, and the repo's gotchas.
  Read it before you start a contribution and before you open a PR.
- **`contributions/`** holds the review and plan for each piece of work, found by its slug.
- **`.claude/agent-memory/architect/`** is the architect's own notes on the codebase:
  where things live, generalisations already made, gotchas. Not status, not decisions.
  **`.claude/agent-memory/orchestrator/`** is the orchestrator's, about splits and waves.
