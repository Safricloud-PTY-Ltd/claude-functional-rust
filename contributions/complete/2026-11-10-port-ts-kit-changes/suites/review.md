# Review: suites — the check's test step selects the normal suite itself

**The ask.** Make step 3 of `cargo xtask check` (`test` in `check_steps`) pass the normal
suite's filter set explicitly, so the gate runs no slow or perf test whatever
`.config/nextest.toml` says; pin the arguments in `check_steps_tests.rs`; hold the filter equal
to `[profile.default] default-filter` with a test. Source: issue #17, through the
orchestrator's brief of 2026-11-10.

## What I found

1. **Step 3 is one line in `check_steps.rs`**:
   `nextest run --workspace --no-tests=pass`. Its selection today comes entirely from the
   nextest config. Nothing else in the repo quotes those arguments (`git grep` outside
   `assets/xtask` finds only the per-edit hook's own `-p` runner, which is a different command).
2. **`-E` alone is intersected with the profile's default filter** (orchestrator's
   measurement). So `-E <normal>` under a config whose default filter is narrower, or matches
   nothing, would run fewer tests than the normal suite, and `--no-tests=pass` would let an
   empty run pass. `--ignore-default-filter -E <normal>` makes the gate's selection exactly
   the normal suite, independent of the config in both directions.
3. **`check_steps` is 36 lines.** Moving the test arguments to a module constant, as step 7's
   already are, keeps it under 40 with room to spare.
4. **xtask has no TOML parser.** The default profile's filter is one basic-string line with
   no escapes, so the test can find it by reading the file from
   `CARGO_MANIFEST_DIR/../.config/nextest.toml` and taking the `default-filter` line of the
   `[profile.default]` table. No new dependency, no NEEDS.
5. **The drift test fails when the config has no default filter.** That is its job, and it
   means the "config deleted" measurement shows a failing check: the failure must be the
   drift test alone, with the planted slow and perf tests not run. I'll measure both the
   check and step 3's command line on its own.

## Directions

- **A. `--ignore-default-filter -E <normal>` (recommended).** The gate's selection is the
  normal suite, exactly, whatever the config says. Cost: the expression lives in two places,
  held together by the drift test.
- **B. `-E <normal>` only.** Still never runs slow or perf. Cost: a config that narrows the
  default filter silently shrinks the gate, and `--no-tests=pass` hides an empty run.
- **C. `--profile default` plus a config check.** Doesn't meet the ask: the selection still
  comes from the config.

## Recommendation

A. The new argument vector is
`nextest run --workspace --no-tests=pass --ignore-default-filter -E <normal filter>`.

## Questions

none
