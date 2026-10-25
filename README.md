# claude-functional-rust

A scaffold for building functional Rust with Claude Code, delivered by one command.

It is not a library and not an application. It is the harness: four agents, a set of write
and git guards, two skills, and a validated Cargo workspace. Together they turn Claude Code
into a team that writes contracts first, tests second and implementations last. The codebase
it produces has one crate per swappable module, failures as `Result` values, and I/O kept in
adapter crates behind ports. It is the Rust sibling of
[claude-functional-typescript](https://github.com/Safricloud-PTY-Ltd/claude-functional-typescript),
and it was taken from a from-scratch video player where it has run efforts of up to a dozen
parallel architects.

## Quick start

In a new or existing project directory:

```bash
npx github:Safricloud-PTY-Ltd/claude-functional-rust
git init          # if it isn't a repository yet
claude            # starts the orchestrator; ask it to "install the scaffold"
```

The command writes `CLAUDE.md`, `CONTRIBUTING.md` and `.claude/`, and touches nothing else.
The first effort then installs the Rust workspace from the `project-scaffold` skill's assets:
the root `Cargo.toml` and `Cargo.lock`, the toolchain pin, the lint and supply-chain configs,
CI, the pre-push hook, `crates/shared` and `xtask`. It fills in `CLAUDE.md`'s placeholders
with you, then ends with `cargo xtask check` green.

```
npx github:Safricloud-PTY-Ltd/claude-functional-rust [directory] [--force]

  directory   Where to write. Defaults to the current directory.
  --force     Overwrite files that already exist. Without it they are left alone.
```

Re-running is safe: existing files are kept, and the report tells you how many were skipped.
Pass `--force` to pull in a newer version of the harness. It only ever touches the paths it
installed, so your crates are never at risk. But `CLAUDE.md` and `CONTRIBUTING.md` are among
those paths, and `--force` will overwrite your project description and your own gotchas.
Commit before you use it.

Requirements:

- **Node 24+**, for the command and for the hooks, which parse their input with node.
- **bash**: Git Bash on Windows.
- **rustup**, which installs the pinned toolchain on first use.
- **cargo-nextest, cargo-deny, cargo-machete and cargo-llvm-cov**:
  `cargo install --locked cargo-nextest cargo-deny cargo-machete cargo-llvm-cov`.
- **`gh`**, for the PR steps.

## What lands in your project

```
CLAUDE.md                     your project's instructions to Claude — fill in the placeholders
CONTRIBUTING.md               PR review (codex or claude, off by default), Codex briefs, target/ hygiene, gotchas
AGENTS.md                     Codex's entry point, as brief runner or reviewer, and the hook rules it keeps itself
.claude/
  settings.json               registers the hooks and makes the orchestrator the default agent
  agents/                     orchestrator, architect, test-writer, implementor
  codex/run-brief.sh          runs a test-writer or implementor brief on Codex (Brief runner: codex, off by default)
  hooks/                      write guard, git guard, territories, post-edit rustfmt/clippy/tests
  skills/
    code-standards/           the rulebook: how every crate, function, error and test is written
    project-scaffold/         the toolchain, plus the workspace it installs, in assets/
```

Nothing else is copied. The scaffold's own working files (this README, `bin/`,
`package.json`, and any agent memory) stay here.

## How the workflow works

Four agents, defined in [.claude/agents/](.claude/agents/), with a deliberate split: two
decide, two write.

- **orchestrator** is the session you talk to (`claude`). It splits an ask into
  workstreams and draws each one a **territory**, a set of files no other agent may write.
  It spawns one architect per workstream in waves, keeps the shared files itself (the root
  `Cargo.toml`, `Cargo.lock`, CI, docs), asks you each wave's questions in one round, routes
  requests between architects, and owns the branch, the PR, review and merge.
- **architect** owns the shape of the code in its workstream. It writes a review and a plan,
  then a **contract** for every function before any of them exist: a signature, its rustdoc
  and a `stub!` body. It dispatches the other two, verifies what comes back, and commits.
  It never writes a function body. Started alone (`claude --agent architect`), it runs a
  whole contribution itself, which suits small projects.
- **test-writer** turns a contract into failing tests (unit, proptest, and a black-box
  `tests/api.rs` per crate), working from the signature and doc comment alone. If it can't
  derive a test, that's a gap in the contract, and it says so rather than guessing.
- **implementor** writes one contract's body until its tests pass. It can't change the
  signature, the doc comment or the tests, and it declares no functions of its own: a
  function it wants is a question back to the architect.

Work runs in phases: leaves first, then the functions that call them. Within a phase, agents
run in parallel on files that don't overlap, all in one checkout on one branch, with no
worktrees. Tests are written and confirmed red (`not implemented: <fn>`) before any
implementation starts, so a passing test is evidence.

Each piece of work gets an id (`<yyyy-mm-dd>-<slug>`) that names its branch and its folder
under `contributions/`. The folder holds a review for you to read and a plan with the call
graph, the phases and every decision made along the way.

### The guards

Hooks in [.claude/hooks/](.claude/hooks/) enforce what prose can't:

- **Write guard.** An architect writes only inside its territory. A test-writer writes only
  `<fn>_tests.rs` beside an existing contract, or `tests/*.rs`. An implementor edits only
  files carrying an `@stub` marker, and creates none. Every file has at most one owner per
  phase.
- **Git guard.** Sub-agents get read-only git. A nested architect may also `git add` and
  `git commit` explicit paths, but only after the guard has asked git which files the command
  would touch and checked each one against the architect's territory.
- **Post-edit check.** After every edit of a `.rs` file: rustfmt on the file, clippy on its
  crate with warnings as errors, and the file's own tests.

An agent that strays is stopped by the harness rather than caught in review.

## The standards

The full rulebook is [code-standards](.claude/skills/code-standards/SKILL.md). The short
version:

- **A module is a crate.** Every crate declares a kind: `core` (pure, no I/O, no third-party
  runtime dependencies), `adapter` (one external thing behind one port, plus its in-memory
  fake), `app` (the composition root) or `tool`.
- Small functions with one job, one concept per file, and the file named after its export.
  At most three parameters and 40 lines per function, and no boolean parameters.
- Failures are `Result` values with narrow, named error types. Production code never
  panics: no `unwrap`, `expect`, panicking index, or narrowing `as`.
- No interior mutability, no globals, no `unsafe` in core crates. Ports are traits declared
  by the consumer; tests use fakes, never mocks.
- Every named function is a contract written first, with `# Parameters`, `# Returns`,
  `# Errors` and `# Remarks` in its rustdoc.

Every one of those that a tool can check is checked by a tool: rustc, clippy (the workspace
lint table and `clippy.toml`), rustfmt, cargo-deny, cargo-machete, or `cargo xtask check`.
That last one runs eight gates in one report: fmt, clippy, tests, doc tests, deny, machete,
no stubs left, and crate purity. The
[project-scaffold](.claude/skills/project-scaffold/SKILL.md) skill installs that toolchain and
documents which tool enforces which rule.

## Working on this repo

The scaffold is the product, so everything here is payload that will land in someone else's
project. A few things follow from that:

- **`bin/cli.mjs` is plain JavaScript on purpose.** npm installs this package under
  `node_modules`, where Node refuses to strip TypeScript types, so a `.ts` file can't be a
  bin. Staying JavaScript keeps the command a single file with no build step and no
  dependencies.
- **`assets/gitignore` and `assets/gitattributes` have no dot.** npm strips `.gitignore` out
  of every tarball it builds, so a dotted name would silently never reach anyone. The
  `project-scaffold` skill renames both on the way in.
- **`.gitattributes` here forces LF**, because `npx github:` installs from a git checkout
  and the bash hooks break on CRLF.
- **`assets/` is a complete workspace.** To validate a change to it, copy it into an empty
  git repository, rename the two files, commit, and run `cargo xtask check`. Then re-run the
  five negative cases in the skill's **Versions** section. Don't build inside `assets/`.

To test a change to the command before it's on `main`, install from the branch:

```bash
npx github:Safricloud-PTY-Ltd/claude-functional-rust#<branch> ../somewhere-empty
```

Agent memory lives in `.claude/agent-memory/` and is deliberately kept out of git here, so
this repo's notes never ship. In a project bootstrapped from the scaffold it's ordinary
committed content, shared with the team. What the source project learned the hard way ships
as the **Gotchas** section of `CONTRIBUTING.md` instead.

## Status

Early. The harness, the standards and the toolchain are in daily use in the project they
came from. From a fresh bootstrap, the starter workspace has been run end to end with
`cargo xtask check`, green and on five deliberate violations. A whole effort, from ask to
merge, has not yet been run on a fresh bootstrap, so expect to fix things on first use.
Open work is tracked in
[issues](https://github.com/Safricloud-PTY-Ltd/claude-functional-rust/issues).
