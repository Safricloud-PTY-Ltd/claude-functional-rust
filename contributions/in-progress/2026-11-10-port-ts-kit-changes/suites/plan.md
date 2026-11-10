# 2026-11-10-port-ts-kit-changes / suites

## Decisions

2026-11-10. No questions for the owner (the owner delegated this effort's choices to the
orchestrator). Direction A of the review: step 3 passes `--ignore-default-filter -E <normal>`,
so its selection is exactly the normal suite whatever `.config/nextest.toml` says.

## Approach

One existing contract changes; no new function, file or crate. `check_steps` keeps its
signature. Its test arguments move to a module constant, `TEST_ARGS`, built from a second
constant, `NORMAL_SUITE_FILTER`, the default profile's `default-filter` character for
character. `check_steps_tests.rs` pins the vector literally and adds a drift test that reads
`CARGO_MANIFEST_DIR/../.config/nextest.toml`, takes the `default-filter` line of the
`[profile.default]` table, and compares it with step 3's last argument.

Call graph: `run` (existing) -> `check_steps` (changed) -> nothing new.

**Loop skipped.** The write guard refuses test-writers and implementors under `assets/` (its
layout arm knows `crates/*` and `xtask/` at the repo root only), and the change is a few lines
in one function. The architect writes contract, tests and body, in that order, and watches
the tests fail against the old body before writing the new one. Built and tested only in the
scratch install, `CARGO_INCREMENTAL=0`.

## Phase 1 — the test step

Contracts: `.claude/skills/project-scaffold/assets/xtask/src/core/check_steps.rs`

- 1.1 `check_steps` — `xtask/src/core/check_steps.rs` — approach: step 3's arguments become
  `nextest run --workspace --no-tests=pass --ignore-default-filter -E <NORMAL_SUITE_FILTER>`;
  doc table row 3 and a paragraph on the filter — calls: nothing new
- 1.2 tests — `xtask/src/core/check_steps_tests.rs` — the third-step test pins the new vector;
  a new test holds the filter equal to the config's default profile filter
- 1.3 `tests/api.rs` — unchanged; it builds its tables from `check_steps()`

Deviations:

- 1.1: the filter isn't the one in the brief. It carries the `^xtask$` anchor and
  `not binary(=xtask)`, mirroring the reserve config as fixed in 0586498 (see below). The doc
  comment explains the anchor in the same words as the config's comment.
- 1.2: the drift test reads the config with a small line reader in the test file,
  `default_profile_filter`, which has its own test. It returns `None` for a value with a quote
  or a backslash in it, so the drift test fails instead of comparing a half-read value.

## Decisions made mid-loop

- 2026-11-10. **nextest refuses a `binary()` matcher that matches no binary in the
  workspace**, for every matcher kind (nextest-filtering `compile.rs`,
  `expect_non_empty_binary_names`). It also compiles every profile's filter on every run, and
  `-E` gets the same check. Measured in the scratch install with cargo-nextest 0.9.145:
  - With the config as committed at 52a0e4f, every nextest run in a fresh scaffold exited 96,
    `cargo nextest run -p xtask` included.
  - With a `_slow` integration binary and no `_perf` one, every run failed on
    `profile.perf`.
  - A step-3 `-E` without the anchor exited 94 in a workspace with neither binary.

  Reported to the orchestrator as a NEEDS. It fixed the config in 0586498 with an `xtask`
  anchor plus `not binary(=xtask)`. Step 3's filter mirrors that, character for character.
- 2026-11-10. **The step keeps nextest's fail-fast default when the config is gone.** With no
  config, `fail-fast = false` goes with it, and step 3 stopped after 32 of 168 tests on the
  drift test's failure. I didn't add `--no-fail-fast`: the ask pins the selection, not the run
  policy, and a missing config already fails the check through the drift test.

## Deferred

none
