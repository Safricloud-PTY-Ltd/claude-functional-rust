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

Deviations:

## Decisions made mid-loop

## Deferred
