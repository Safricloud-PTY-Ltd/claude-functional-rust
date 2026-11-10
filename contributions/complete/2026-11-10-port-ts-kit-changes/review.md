# Review: port the TypeScript kit's changes of 2026-11-10

**The ask.** Port into this kit what claude-functional-typescript gained on 2026-11-10 (its PRs
#34 and #38), as issues #14 to #20 here describe it. Source: the owner, 2026-11-10, who also
said to work without stopping for questions and to choose for the completeness of a
bootstrapped project. So every decision below is made, not asked, and each is listed again in
the PR under **Decisions to veto**.

## What the ask involves

Seven issues. Six are harness and prose with no Rust in them. One (#17) changes one Rust
function in the scaffold's `xtask`.

| Issue | What lands | Rust |
| :-- | :-- | :-- |
| #14 hook tests | `node:test` suites for the write guard, the per-edit hook, the git guard and the territory reader, plus a fix for whatever they find | no |
| #15 `gh` help | sub-agents may read `gh` help and a project board | no |
| #16 labels | `blocked-on-product-owner` and `blocked-on-dev-owner` replace `blocked-on-owner` | no |
| #17 suites | normal, slow and perf test suites; only the normal one runs in `cargo xtask check` and pre-push | the check's test step |
| #18 designer | a designer agent, its fence in the write guard, and when the main session runs it | no |
| #19 board | a `Project` setting and the steps that keep a GitHub Project | no |
| #20 lessons | new gotchas in `CONTRIBUTING.md` | no |

## What I measured before splitting

All on this machine: Windows 11, cargo 1.98.1, cargo-nextest 0.9.145, gh 2.89, in a scratch
install of the scaffold's workspace with one normal, one slow and one perf test planted as
both unit and integration tests.

- **The suites mechanism works with nextest profiles.** With a `default-filter` on each
  profile in `.config/nextest.toml`, the default profile collected 167 tests and none of the
  four planted slow or perf tests, `--profile slow` and `--profile perf` collected only their
  own two, and `--profile ci` collected normal plus slow (169 of 171).
- **A slow test runs alone by name under its profile**, and the same name with no profile
  exits 4, "no tests to run". A misspelt profile exits 96 and lists the known ones. So a
  wrong selection fails loudly; it never silently runs the normal suite.
- **`-E` is bounded by the profile's default filter**, so no filter typed by hand reaches a
  slow or perf test under the default profile.
- **Plain `cargo test` has no filter sets.** `cargo test -p shared` ran all three suites. The
  per-edit hook falls back to `cargo test` when nextest isn't installed, so the hook must
  skip slow and perf files by name, not rely on the profile.
- **The write guard refuses a test-writer and an implementor under `assets/`**
  (`may only write .rs files under a crate's src/ or tests/`). Its layout arm knows `crates/*`
  and `xtask/` at the repo root only. In this repo the scaffold's `xtask` is not at the root,
  so an architect's loop cannot dispatch sub-agents on it.
- **`gh`:** `gh issue list -h` prints help, but `-h` takes a value under `gh auth`, `gh config`
  and `gh repo create|edit` (hostname, host, homepage). Help wins over a later flag
  (`gh label list --help --limit 1` printed help).
- **The board's read query** (`repository { projectsV2 }`) runs with this token and returns
  no linked project for this repo.

Still running when this was written: coverage under `--profile ci`, and which suite
`cargo mutants` runs. The plan records them.

## The split

One workstream. Everything else is reserve, edited by the orchestrator.

### Workstream `suites` (wave 1)

- **Territory:** `.claude/skills/project-scaffold/assets/xtask/*` and its own folder.
- **Delivers:** the check's third step selects the normal suite itself, with an explicit
  filter set, so `cargo xtask check` cannot run a slow or perf test whatever the nextest
  config says. Its tests pin the arguments, and pin that they agree with the default
  profile's filter in `.config/nextest.toml`.
- **Calls:** nothing from another workstream. It reads the reserve file
  `.config/nextest.toml`, which is written before it starts.
- **How it runs:** the change is a few lines in one function, and the guard keeps
  sub-agents out of `assets/`, so the architect writes the contract, the test and the body
  itself, red before green, in a scratch install of the workspace. It says so in its report.

### Reserve edits (the orchestrator's)

- **Hooks.** `check-file.sh`: real paths before the repo-relative path (#14), and no test run
  for a slow or perf file (#17). `guard-writes.sh`: a slow or perf test sits beside the same
  contract (#17), and a designer's fence (#18). `git-policy.cjs`: help and project reads
  (#15). New: four test files and one shared test helper (#14).
- **Agents.** New `designer.md`. `orchestrator.md` and `architect.md`: the designer, the
  board, the labels, the suites, and `designer` on both spawn lists. `test-writer.md` and
  `implementor.md`: the by-name run of a slow or perf test.
- **Scaffold assets that aren't Rust.** `.config/nextest.toml` (the profiles), `ci.yml` (the
  coverage job runs `--profile ci`), `.githooks/pre-push` (a comment).
- **Skills.** `code-standards` (Tests), `project-scaffold` (what enforces the rule, what was
  measured, what an installed project copies by hand).
- **Top level.** `CLAUDE.md` (the `Project` setting, the commands, the designer),
  `CONTRIBUTING.md` (the board's commands, the lessons of #20), `AGENTS.md` (the
  test-writer's rule for Codex), `README.md`.

## Decisions made on the owner's behalf

1. **The board goes into the orchestrator's own steps and the solo architect's, not into a
   ported git manager (#19).** The owner's decision in the TypeScript kit was that whoever
   already owns issues, PRs and the merge keeps the board. Here that is the main session.
   Porting the git manager would rewrite every git step in two agent files and add a sixth
   policy to the git guard, which none of the seven issues asks for. The commands live once,
   in `CONTRIBUTING.md`, and both agent files point there. Cost: the main session's context
   carries the board work. Forecloses nothing; a git manager can take the section over later.
2. **Suites are told apart by file and module name, and chosen by nextest profile (#17).**
   `<name>_slow_tests.rs` mounted as `mod slow_tests`, `<name>_perf_tests.rs` as
   `mod perf_tests`, and `tests/<name>_slow.rs`, `tests/<name>_perf.rs`. `#[ignore]` gives two
   buckets and can't tell slow from perf.
3. **The check names its selection too.** The profile protects every plain `cargo nextest
   run`. The explicit filter in the xtask step protects the gate when the config is missing
   or edited, and is what a Rust test can pin. Two places for one expression is the cost; a
   test holds them together.
4. **No new timeout in the nextest config.** The rule moves a test that needs a raised
   timeout to the slow suite. Enforcing a ceiling on the normal suite would be a new rule
   the owner hasn't asked for, and a timeout under load is the failure this change removes.
5. **Help is honoured for built-in `gh` commands only, and `-h` not under `auth`, `config` or
   `repo` (#15).** An alias or an extension gets its arguments raw, so `gh <alias> --help`
   can run anything. Sub-agents also get `gh project list|view|item-list|field-list`.
6. **The hook tests find Git Bash themselves on Windows.** The TypeScript kit asks you to run
   them from Git Bash, because from PowerShell `bash` can be WSL's. Here the helper asks git
   where it is installed and uses the `bash` beside it.
7. **This effort's folder is committed.** No earlier effort in this repo kept a
   `contributions/` folder. This one has an architect, which needs one. The folder isn't in
   the installer's payload, so it never ships.
8. **The PR is opened and promoted, not merged.** `CLAUDE.md` here still carries the
   Auto-merge placeholder, and `main` has needed `--admin` before. That yes is the owner's.
9. **The `Project` setting ships as `yes | no`**, to be filled in with the other settings
   when a project installs the scaffold, as in the TypeScript kit.

## Alternatives to this split

- **No workstream: put the selection in `.config/nextest.toml` alone.** No Rust changes, no
  architect. Cost: nothing pins the gate's selection, and a project whose config lacks the
  filter runs every suite in its check without saying so. Rejected for that.
- **Port the git manager first, then the board (#19's other option).** Closest to the
  TypeScript text. Cost: the largest change in the effort, to the part of this kit that
  works, with a new agent to probe. Rejected as above.
- **Give the hooks to an architect too.** They are bash and JavaScript; the architect's loop
  is built for Rust contracts. The orchestrator's rules put `.claude/` in the reserve.

## Not in scope

- Issues #9 to #13, which are open here and not part of the port.
- A run of the designer's three turns, or of an effort with `Project: yes`. Neither kit has
  done one. The PR says what was probed and what wasn't.
