---
name: project-scaffold
description: Installs, describes and repairs the toolchain that enforces this repo's code standards — the pinned Rust toolchain, the workspace lint table, clippy.toml thresholds, rustfmt, cargo-nextest, proptest, cargo-deny, cargo-machete, cargo-llvm-cov coverage, the `cargo xtask check` runner and its crate-purity rule, the git pre-push hook, Dependabot, and GitHub Actions CI. Use this when the repo has no root Cargo.toml yet, when `cargo xtask check` is missing or broken, when adding a crate to the workspace, when adding or changing any lint, test, or CI configuration, when a tool in the chain is upgraded, or when someone asks which tool enforces a given rule.
---

# Project scaffold

The `code-standards` skill says what the code looks like. This skill installs the tools that
refuse code that doesn't. Everything in `assets/` was validated together (see **Versions**):
a Cargo workspace with the `shared` crate (`stub!`) and `xtask` (`cargo xtask check`, with its
own contracts and tests), and the configs around them. Copy them as they are, then adjust
with a reason. Once installed, the files at the repo root are the source of truth; `assets/`
is only the starting point.

## What each tool enforces

| Standard | Enforced by |
| :-- | :-- |
| A crate's `pub` surface is its only door; who may depend on whom | rustc; each crate's `Cargo.toml` |
| A `core` crate depends at runtime only on workspace `core` crates; every crate declares a kind | `cargo xtask check`, step `purity` (`[package.metadata.standards] kind`) |
| No `unsafe` | `[workspace.lints.rust] unsafe_code = "deny"`, plus `#![forbid(unsafe_code)]` in every core `lib.rs` |
| No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `unreachable!`, panicking index or string slice, `process::exit` | `[workspace.lints.clippy]` restriction lints, `deny` |
| No `dbg!`, `println!`, `eprintln!` outside an allowed `main.rs` | clippy `dbg_macro`, `print_stdout`, `print_stderr` |
| Results must be handled | rustc `unused_must_use = "deny"`; clippy pedantic `must_use_candidate` |
| No narrowing `as`, no wildcard imports, docs state `# Errors` | clippy `pedantic` (warn, and warnings are errors) |
| No interior mutability or locks in the functional core | `clippy.toml` `disallowed-types` |
| At most 3 parameters, 40 lines per function, no boolean parameters, bounded complexity | `clippy.toml` thresholds |
| `<concept>/mod.rs`, never `<concept>.rs` beside a directory | clippy `self_named_module_files` |
| Doc comment on every public item | rustc `missing_docs = "deny"` |
| One formatting | `rustfmt.toml`, `cargo fmt --all --check` |
| No unused dependencies | `cargo machete` |
| Licences, advisories, yanked crates, unknown registries | `cargo deny check` (`deny.toml`) |
| No stub reaches `main` | `cargo xtask check`, step `no-stubs` |
| Tests red before green; properties | `cargo nextest`, `proptest`; the loop itself |
| A test that depends on the clock or the machine can't refuse a push: slow tests run in CI only, perf tests by hand only | the profiles in `.config/nextest.toml` (`default`, `slow`, `perf`, `ci`); the `test` step of `cargo xtask check`, which passes the normal suite's filter itself |
| Line coverage floor, over the normal and slow suites | CI job `coverage`: `cargo llvm-cov nextest --profile ci --fail-under-lines` |
| Tests that assert something | the architect's deliberate breaks at phase verification; `cargo mutants` by hand when wanted (`.cargo/mutants.toml` keeps it on the normal suite) |
| Nothing is pushed unchecked | `.githooks/pre-push`, CI `cargo xtask check` on Linux and Windows |
| Who may write which file during the loop, territories and the designer's folder included; post-edit lint and tests | `.claude/hooks/guard-writes.sh`, `check-file.sh` |
| Which git and gh commands a sub-agent may run (read-only; a nested architect also `add` and `commit` of files in its territory) | `.claude/hooks/guard-git.sh` → `git-policy.cjs`, `territory.cjs` |
| The hooks do what this table says | `node --test ".claude/hooks/*.test.cjs"`, by hand after any change to a hook |

## Procedure for a new repository

This is a copy, not code, so the main session does it directly, with no loop (the
orchestrator's **Installing the scaffold**). Do it on a `chore/<yyyy-mm-dd>-scaffold` branch.

1. Copy everything in `assets/` to the repo root, preserving paths (dot-directories
   included: `cp -r .claude/skills/project-scaffold/assets/. .`). Then rename `gitignore` to
   `.gitignore` and `gitattributes` to `.gitattributes`. They ship undotted because npm strips
   a dotted `.gitignore` out of every tarball it builds, so the dotted name would never
   survive `npx`. Where the repo already has either file, merge the lines in instead.
2. Install the toolchain once per machine and enable the hook once per clone:

   ```bash
   rustup install                                # installs the toolchain pinned in rust-toolchain.toml
   cargo install --locked cargo-nextest cargo-deny cargo-machete cargo-llvm-cov
   git config core.hooksPath .githooks
   ```

   `cargo-mutants` is optional, for mutation testing by hand.
3. Commit the copied files, then run `cargo xtask check`. The `no-stubs` step searches with
   `git grep`, so it needs a git repository. All eight steps pass on the starter tree. If one
   fails here, the environment differs from the validated one: fix the environment before
   touching the configs.
4. Fill in `CLAUDE.md`'s placeholders with the owner: the project paragraph, the settings,
   the build and deploy commands, and the PR link format. Add the project's proper nouns to
   `clippy.toml`'s `doc-valid-idents` as they appear.
5. Commit, then open, review and merge the scaffold like any contribution. The next effort
   starts the real work.

## Procedure for an existing repository

Copy the configs one at a time, running the matching check after each (`cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check`, ...). Give every
crate a `[package.metadata.standards] kind` and `[lints] workspace = true`, then add `xtask`
and `crates/shared`. Expect the lint table and the purity step to fail on legacy code: that's
information for the architect's review, not something to silence. Relax a lint only with an
`#[allow(..., reason = "...")]` at the item, or a dated comment in `Cargo.toml` saying which
contribution turns it back on.

## Adding a crate

1. `crates/<name>/Cargo.toml`: inherit `edition`, `rust-version`, `publish` from the workspace,
   set `[package.metadata.standards] kind`, and add `[lints] workspace = true`. A crate
   without the lints line silently escapes the whole rulebook, so the purity step checks for
   the kind and review checks for the lints line. Write it first, together with `src/lib.rs`:
   the workspace takes `crates/*` as members, so a directory with no manifest, or a manifest
   with no target, breaks every cargo command in the checkout.
2. `src/lib.rs`: a `//!` summary, `#![forbid(unsafe_code)]` for a core crate, then `mod` and
   `pub use` lines only.
3. `tests/api.rs`: the black-box suite.
4. If other crates will depend on it, add it to the root `[workspace.dependencies]` as a path
   dependency so every consumer writes `{ workspace = true }`.
5. An adapter crate that links a system library adds its install step to both CI jobs.

## Decisions baked into the configs

- **Module = crate.** Boundaries are enforced by the compiler instead of by regexes over import
  paths. The cost is more `Cargo.toml` files; the purity step keeps them honest.
- **Lints live once, in the root `[workspace.lints]`.** `pedantic` is on at `warn` with priority
  -1 so individual lints can override it, and every check runs with `-D warnings`, so a warning
  locally is an error in the hook and in CI.
- **`stub!` passes clippy on purpose.** The panic lint does not look inside a macro from another
  crate, which is what lets a workspace full of stubs stay green during the loop. The `no-stubs`
  step is what stops one from merging. Its search is line-anchored (`^[[:space:]]*stub!\(`), so
  a string or a doc that mentions the macro doesn't match. It searches untracked files too
  (`--untracked`; ignored files stay out), since a new contract file is untracked mid-phase. It
  skips `crates/shared`, which defines and tests the macro, and `.claude`, where this skill's
  template copy of `crates/shared` lives.
- **Tests are exempt from the no-panic lints** through the `allow-*-in-tests` keys in
  `clippy.toml`; panicking is how a test fails. The keys cover `#[test]` functions only, not
  helpers in test files.
- **Test files are siblings, not inline modules.** `#[path = "<fn_name>_tests.rs"] mod tests;`
  keeps the test-writer and the implementor in different files, which the write guard relies on.
- **The toolchain is pinned** to an exact stable release so a lint added upstream cannot turn
  `main` red overnight. Bumping it is a contribution: bump, run the check, fix what it finds.
- **`Cargo.lock` ships** with the versions that were validated together. Commit it.
- **Debug info is line tables only** (`[profile.dev]`). Full debug info filled `target/` to
  300 GB under a dozen parallel agents; tests and backtraces need only file and line.
- **Coverage is not in `cargo xtask check`.** It rebuilds the workspace with instrumentation,
  so it runs in CI only, and the loop stays fast. There is no nightly workflow; mutation
  testing is the architects' deliberate breaks, and `cargo mutants` by hand.
- **Three test suites, chosen by nextest profile.** `.config/nextest.toml` gives each profile
  a `default-filter`. With no profile a run is the normal suite: every test that isn't in a
  `slow_tests` or `perf_tests` module or in a `tests/<name>_slow.rs` or `tests/<name>_perf.rs`
  binary. `--profile slow` and `--profile perf` run one suite each, and `--profile ci` the
  normal and slow suites together.
  - `cargo xtask check` (and so pre-push and CI's check job) is the normal suite. Its `test`
    step passes the default profile's filter itself, so the gate holds in a workspace whose
    nextest config is missing or edited, and an xtask test holds the two expressions equal.
  - CI's coverage job runs `--profile ci`. That is where the slow suite runs, once, and it
    lets code only a slow test reaches count toward the floor.
  - No hook and no workflow runs `--profile perf`.
  - A wrong selection is loud. A misspelt profile exits 96 and lists the known ones, and a
    slow test named with no profile exits 4, "no tests to run". Nothing falls back to the
    normal suite in silence.
  - Plain `cargo test` has no profiles and runs every suite. The kit runs tests through
    nextest everywhere. The per-edit hook, which falls back to `cargo test` on a machine
    without nextest, skips slow and perf files by name for that reason.
  - Every `binary()` in the filters also names `xtask` and then excludes it. nextest refuses
    a `binary()` that matches no binary, and compiles every profile's filter on every run,
    so without that a workspace with no slow integration test yet fails every run.
  - Mutation testing stays on the normal suite (`.cargo/mutants.toml`, `test_tool =
    "nextest"`). cargo-mutants' own default is `cargo test`, which runs all three.
- **A designer's folder needs no ignore line.** Nothing in `cargo xtask check` reads
  `contributions/`: `cargo fmt` and clippy take the workspace's crates, and the stub search
  takes `*.rs`.
- **CI runs once per PR.** PRs stay drafts through their review loop, and `ci.yml` skips
  drafts and triggers on `ready_for_review`.
- **Hooks normalise paths.** Claude Code on Windows reports `C:\...` paths and cargo prints
  backslashes; both hooks convert to `/c/...` forward-slash form before comparing anything.
  `check-file.sh` does nothing until the repo has a root `Cargo.toml`.
- **`machete` is started as `cargo-machete`, not `cargo machete`.** The tool decides whether
  cargo invoked it by looking for `CARGO_PKG_NAME`, which `cargo run` always sets, so under
  `cargo xtask` the subcommand form misreads its own name as a directory to scan.
- **`check` captures each step's output and prints it only on failure**, after all steps have
  run; a failed step does not stop the ones after it, so one run shows every problem.
- **Territories are runtime state.** The orchestrator writes `.claude/territories/<workstream>`
  (glob lines and `agent <id>` lines). It's gitignored like `.claude/locks/`. No hook
  clears either: a session started for a quick question in the same checkout would wipe
  a live effort's state. The orchestrator runs `.claude/scripts/clear-effort-state.sh`
  after the merge, or at the start of an effort once the owner confirms that the
  leftovers it found belong to no running effort. The guards key on `agent_id`, which only a sub-agent
  has. The main session carries an `agent_type` (from the `agent` setting) but never an
  `agent_id`, so the orchestrator, or a solo architect, is never fenced. `territory.cjs` is
  the one reader of territory files. It canonicalises paths (resolving `..` and symlinks),
  and both guards use its verdict.
- **The git guard runs on every Bash command** and denies by default. `git-policy.cjs`
  tokenises the command, follows `cd` and `-C`, and refuses git or gh hidden behind `xargs`,
  `env`, a shell `-c` string, `$(...)` or a heredoc. It allows read-only git verbs and
  read-only gh (views, lists, diffs, checks, a project board's lists, GET-only `gh api`, and
  `--help` as the first flag of a built-in command; `-h` too, except under `auth`, `config`
  and `repo`, where gh gives it a value). A nested architect's `add`
  and `commit` are allowed only after asking git (`diff --name-only HEAD`, the same with
  `--cached`, `ls-files --others`) which files the command would touch and checking each
  against the territory. Those two also refuse directories, git pathspec patterns and any
  word bash would rewrite after the check (`$`, globs, braces, `~`), since each could take in
  a file the check never saw. It fences mistakes by cooperative agents. It isn't a sandbox.
- **Hooks are launched as `bash "<path>"`**, with no `args`. With `"args": []` Claude Code
  runs the script itself instead of through a shell, which can't start a `.sh` on Windows
  and needs the execute bit elsewhere, and a hook that fails to start doesn't block: the
  guards silently did nothing. The node helpers are `.cjs` so a project whose `package.json`
  says `"type": "module"` still loads them as CommonJS.
- **Hook input is parsed with node**, not jq, because node is already required by Claude Code
  installs and jq is not present on a stock Windows machine. The hooks need `bash` and `node`
  on the PATH (Git Bash provides the first on Windows).
- **The write guard knows the layout.** Sub-agents may write only under `crates/*/src`,
  `crates/*/tests`, `xtask/src` and `xtask/tests`. A project that keeps crates elsewhere updates the
  `case` in `guard-writes.sh`. A designer is the exception: it writes only inside
  `contributions/in-progress/<id>/design/`. An agent type the guard doesn't name isn't
  fenced by it at all.
- **The per-edit hook resolves real paths.** It takes the edited file's repo-relative path
  from `territory.cjs`, as the guards do. A prefix strip hid every clippy error from
  sub-agents when the project directory arrived in a Windows short name and the file in
  the long one.

## Upgrading an installed project

`npx … --force` brings a newer harness: `.claude/` (agents, hooks, skills, this folder's
`assets/`), `CLAUDE.md`, `CONTRIBUTING.md` and `AGENTS.md`. It doesn't touch the files this
skill installed at the root, so a change to `assets/` reaches an installed project only
when someone copies it. What each change needs, by the date it landed here:

- **2026-11-10, the three test suites.** The upgrade brings the hooks' and the agents' side
  of it. Until the root files follow, a test in `<name>_slow_tests.rs` or
  `<name>_perf_tests.rs` still runs in every `cargo nextest run` and in `cargo xtask check`.
  Copy from `assets/`, merging where the project has changed a file:
  1. `.config/nextest.toml`: the `default-filter` line of `[profile.default]`, and the
     `slow`, `perf` and `ci` profiles. A project with no crate named `xtask` puts one of its
     own binaries' names where the filters say `xtask`.
  2. `xtask/src/core/check_steps.rs` and `check_steps_tests.rs`: the `test` step's
     arguments, and the tests that pin them.
  3. `.github/workflows/ci.yml`: `--profile ci` on the coverage command.
  4. `.cargo/mutants.toml`, and the comment in `.githooks/pre-push`.

  Then run `cargo xtask check`.
- **2026-11-10, the designer, the project board and the two `blocked-on-` labels.** Nothing
  to copy: they live in `.claude/`, `CLAUDE.md` and `CONTRIBUTING.md`. `--force` overwrites
  those last two, so carry your own settings and gotchas over, and add the **Project** line
  to your settings.

## Versions

Validated on 2026-10-25 on Windows 11 with rustc 1.98.1, clippy and rustfmt from the same
toolchain, and cargo-nextest, cargo-deny and cargo-machete at the versions
`cargo install --locked` resolved that day. With the assets installed as the procedure above
says, `cargo xtask check` ran green, all eight steps, then failed at the right step and with
exit code 1 on each of five deliberate violations: an `unwrap` in a core crate (`clippy`), a
third-party dependency in a core crate (`purity`, and `machete` since it is unused), a crate
with no kind (`purity`), a leftover `stub!` in an untracked file (`no-stubs`), and a
misformatted file (`fmt`).

The hooks are the ones in use in the project this scaffold was taken from, where on
2026-09-24 they were exercised on 104 cases of hand-built hook input: 30 for the write guard
(every territory branch, `..` traversal, mixed-case and forward-slash paths, CRLF territory
files, overlaps for all three agent types) and 74 for the git guard against a scratch
repository (read-only git, every other verb, hidden git, and the architect's `add` and
`commit` with own, sibling and reserve files). One VS Code extension session there was seen
not to run project hooks at all, so don't take a quiet guard in that session as proof it
allowed something; test a hook with hand-built input.

The three test suites were added on 2026-11-10, after the sister TypeScript kit, where one
slow UI test had refused three pushes in a row under the suite's own load. Measured in one
place: scratch installs of these assets on Windows 11 (cargo 1.98.1, cargo-nextest 0.9.145,
cargo-llvm-cov 0.9.1, cargo-mutants 27.1.0), with a slow and a perf test planted in
`crates/shared`, each as a unit test and as an integration test.

- `cargo nextest list --workspace` collected the starter tree's tests and none of the four
  planted ones under the default profile, only the two slow ones under `--profile slow`,
  only the two perf ones under `--profile perf`, and the starter tree plus the slow two
  under `--profile ci`. The same held with one or with neither integration binary present.
- A slow test ran alone by name under its profile. The same name with no profile exited 4,
  "no tests to run", or 0 with `--no-tests=pass`, having run nothing. A misspelt profile
  exited 96.
- `binary(/_slow$/)` alone exited 96 in a workspace with no such binary, from the config,
  and 94 as `-E`. With `xtask` named and excluded again it passed, and an integration test
  named `tests/xtask.rs` stayed in the normal suite.
- `cargo xtask check` passed, all eight steps, with four slow and perf tests present that
  fail on purpose, and on a clean install. With the filter taken out of the `test` step
  and the nextest config deleted, the same four failed the step. With the config deleted
  and the step as shipped, no planted test ran, and the step failed only on the xtask test
  that holds the two filters equal.
- `cargo llvm-cov nextest --workspace --profile ci`, with CI's other arguments, ran under
  the `ci` profile and collected the slow tests; with no profile it didn't.
- `cargo mutants` on a function whose only telling test was a slow one: caught with its
  default `cargo test`, missed with `test_tool = "nextest"`, from the flag and from
  `.cargo/mutants.toml` alike.
- `cargo test -p shared` ran every suite.

Not measured: Linux, and the CI workflow itself. Without a nextest config, nextest's own
fail-fast default returns, so a failing run of the `test` step stops at its first failure.

The hooks have had their own tests since 2026-11-10 (`.claude/hooks/*.test.cjs`, 48 cases).
Writing them reproduced a fault on Windows: with the project directory spelled with a
short name and the file with the long one, the per-edit hook hid the edited file's clippy
errors from the sub-agent that made them.

When the toolchain or a tool's major version changes, re-run `cargo xtask check`, re-check the
five negative cases, re-run the suite measurements above and the hooks' tests, and update
this section.

## Files

```
assets/
  Cargo.toml                 workspace members, [workspace.lints], [workspace.dependencies], dev profile
  Cargo.lock                 the validated versions
  rust-toolchain.toml        pinned channel and components
  rustfmt.toml  clippy.toml  deny.toml
  .cargo/config.toml         the `cargo xtask` alias
  .cargo/mutants.toml        cargo-mutants runs through nextest, so on the normal suite
  .config/nextest.toml       nextest profiles: default (the normal suite), slow, perf, ci (normal + slow)
  gitattributes              copied to the root as .gitattributes; LF everywhere, since bash hooks break on CRLF
  gitignore                  copied to the root as .gitignore; target/, .claude/locks/, .claude/territories/
  .githooks/pre-push         cargo xtask check (the normal test suite only)
  .github/dependabot.yml     weekly, grouped: cargo and actions
  .github/workflows/ci.yml   check on Linux and Windows; coverage floor, which runs the slow suite; never the perf suite
  crates/shared/             stub!
  xtask/                     cargo xtask check: fmt, clippy, test (the normal suite), doctest, deny, machete, no-stubs, purity
```
