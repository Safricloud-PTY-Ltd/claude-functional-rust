# 2026-11-10-port-ts-kit-changes

## Decisions

2026-11-10, made by the orchestrator on the owner's instruction to work without questions
and choose for the completeness of a bootstrapped project. Grounds are in the review.

1. The board lives in the orchestrator's steps and the solo architect's; its commands live
   in `CONTRIBUTING.md`. The git manager is not ported.
2. Suites: `<name>_slow_tests.rs` (`mod slow_tests`), `<name>_perf_tests.rs`
   (`mod perf_tests`), `tests/<name>_slow.rs`, `tests/<name>_perf.rs`; chosen by nextest
   profile (`default`, `slow`, `perf`, `ci`).
3. The xtask test step also passes the normal suite's filter set explicitly.
4. No new timeout in the nextest config.
5. `gh` help for built-in commands only; `-h` not under `auth`, `config`, `repo`. Sub-agents
   may read a project board.
6. The hook tests find Git Bash themselves on Windows.
7. This effort's folder is committed.
8. The PR is promoted, not merged.
9. `Project` ships as `yes | no`.

## Approach

One Rust workstream and a large reserve. The workstream changes one function in the
scaffold's `xtask`; it calls no other workstream. The orchestrator writes
`.config/nextest.toml` first, because the workstream's tests read it, then edits the rest of
the reserve while the workstream runs. No reserve file is shared with the workstream.

Order inside the reserve, each step verified before the next:

1. Hook tests against the hooks as they are (#14), to see what is already wrong.
2. Hook changes, each with its test red first: real paths in `check-file.sh`, the suite arms
   in both hooks, the designer's fence, `gh` help and project reads.
3. Agent files, skills, `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`, `README.md`.
4. A walk through each new flow from start to finish (the designer's three turns, the
   board from the effort issue to the merge, a slow test from brief to CI) before the PR.
5. Confirmation in a project bootstrapped from the pushed branch, on Windows and in a Linux
   container.

Workstream graph: `suites` reads `.config/nextest.toml` (reserve). Nothing reads `suites`.

## Reserve

- `.claude/skills/project-scaffold/assets/.config/nextest.toml`: the four profiles. For
  `suites`, and for every plain nextest run.
- `.claude/skills/project-scaffold/assets/.github/workflows/ci.yml`, `.githooks/pre-push`:
  where each suite runs.
- `.claude/hooks/check-file.sh`, `guard-writes.sh`, `git-policy.cjs`; new
  `hook-test-kit.cjs`, `territory.test.cjs`, `guard-writes.test.cjs`, `check-file.test.cjs`,
  `git-policy.test.cjs`.
- `.claude/agents/designer.md` (new), `orchestrator.md`, `architect.md`, `test-writer.md`,
  `implementor.md`.
- `.claude/skills/code-standards/SKILL.md`, `.claude/skills/project-scaffold/SKILL.md`.
- `CLAUDE.md`, `CONTRIBUTING.md`, `AGENTS.md`, `README.md`.

## Wave 1 — the check's test step

- W1.1 suites — territory: `.claude/skills/project-scaffold/assets/xtask/*`,
  `contributions/in-progress/2026-11-10-port-ts-kit-changes/suites/*` — ask: the check's
  third step selects the normal suite with an explicit filter set, pinned by its tests and
  held equal to the default profile's filter — depends on: none

Deviations: the architect made the change itself, without T and I briefs. The write guard
refuses a test-writer and an implementor under `assets/` (measured before the spawn), and
the change is a few lines in one function. It kept contract, then red tests, then body.

## Decisions made mid-loop

All 2026-11-10.

1. **Every `binary()` matcher in the nextest profiles names `xtask` and excludes it again.**
   The architect found that the config committed in 52a0e4f failed every nextest run in a
   fresh install: nextest refuses a `binary()` that matches no binary, and compiles every
   profile's filter on every run. My first measurement planted both integration binaries
   and never met the empty case. Reproduced (exit 96, whole workspace and `-p shared`),
   fixed in 0586498. Grounds for the longer form over the architect's shorter one: with
   the short form, a file named `tests/xtask.rs` would have left the normal suite silently.
   Measured in four workspace states.
2. **The xtask step passes `--ignore-default-filter` with its `-E`** (the architect's
   decision). `-E` alone is intersected with the config's filter, so a narrower config
   would shrink the gate without a word.
3. **No `--no-fail-fast` on the step** (the architect's). With no nextest config at all,
   nextest's fail-fast default returns and a failing run stops early. The check still fails
   as it should. The ask pinned the selection, not the run policy.
4. **`.cargo/mutants.toml` with `test_tool = "nextest"`.** Measured: cargo-mutants' default,
   `cargo test`, runs all three suites; with nextest it runs the normal one. Mutation
   testing by hand stays on the normal suite, as in the TypeScript kit.
5. **The per-edit hook takes its repo-relative path from `territory.cjs`.** The Windows
   short-name fault that #14 offered only as a reason to write the test reproduced here, on
   the first run of the new test. One resolver for every hook, not a second one.
6. **A new agent type is not spawnable in the session that writes its file.** Measured: this
   session refused `designer` ("Agent type 'designer' not found"). The TypeScript kit's
   lesson said the opposite of its own session. `CONTRIBUTING.md` records both, and the
   designer was probed in a fresh headless session in a bootstrapped project instead: it
   spawned, its tools were `Read, Write, Edit, Grep, Glob, Skill, WebFetch, WebSearch,
   Artifact, ArtifactComments, ArtifactData`, a write in its design folder succeeded, and a
   write at the project root was refused with the guard's message.
7. **The effort issue's title starts with the effort's id.** Not in the TypeScript text.
   Walking the board's flow showed that a restarted session, before any plan exists, could
   not tell its effort issue from one an abandoned effort left open.
8. **The designer's `.prettierignore` rule is not ported.** Nothing in `cargo xtask check`
   reads `contributions/`. The skill says so.
9. **Issue #12 stays open.** #17 asked to answer it together with where a slow sibling's
   shared helper lives. The sibling's answer is in the standards (`test-support`, or the
   crate's `mod testing`). #12's own case is a helper over a `tool` crate's types shared
   with `tests/api.rs`, which `test-support` may not depend on, and it asks for a change to
   xtask. That is its own contribution.
10. **The hook tests ship in `.claude/hooks/`**, as they do in the TypeScript kit, so a
    project that adapts a hook can run them.

## Deferred

None.
