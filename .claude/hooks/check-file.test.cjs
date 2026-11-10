// Tests for check-file.sh: what an edited .rs file gets formatted, linted and tested with, and
// what a sub-agent is shown. The hook's tools are stubs first on PATH that record their
// arguments, so no run reaches the machine's own cargo or rustfmt. Every test checks that the
// clippy stub was called, which is what proves the stubs were the ones found.
// Run: node --test .claude/hooks/check-file.test.cjs
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { makeProject, write, otherSpelling, stubTools, runHook } = require('./hook-test-kit.cjs');

// cargo: records its arguments. `nextest --version` exits $STUB_NEXTEST (0: installed). clippy
// prints $STUB_CLIPPY_OUT and exits $STUB_CLIPPY_EXIT. rustfmt: records, exits $STUB_RUSTFMT_EXIT.
const TOOLS = {
  cargo: [
    '#!/bin/sh',
    'printf \'%s\\n\' "$*" >> "$STUB_LOG"',
    'if [ "$1 $2" = "nextest --version" ]; then exit "${STUB_NEXTEST:-0}"; fi',
    'if [ "$1" = clippy ]; then printf \'%s\' "${STUB_CLIPPY_OUT:-}"; exit "${STUB_CLIPPY_EXIT:-0}"; fi',
    'exit 0',
    '',
  ].join('\n'),
  rustfmt: '#!/bin/sh\nprintf \'rustfmt %s\\n\' "$*" >> "$STUB_LOG"\nexit "${STUB_RUSTFMT_EXIT:-0}"\n',
};

const CLIPPY = 'clippy --quiet -p orders --all-targets --message-format=short -- -D warnings';
const SRC = 'crates/orders/src';
const FILES = {
  'Cargo.toml': '[workspace]\nmembers = ["crates/*"]\n',
  'crates/orders/Cargo.toml': '[package]\nname = "orders"\nversion = "0.0.0"\n',
  [`${SRC}/lib.rs`]: '//! orders\n',
  [`${SRC}/types.rs`]: '',
  [`${SRC}/line_total.rs`]: 'pub fn line_total() {}\n',
  [`${SRC}/line_total_tests.rs`]: '',
  [`${SRC}/line_total_slow_tests.rs`]: '',
  [`${SRC}/line_total_perf_tests.rs`]: '',
  [`${SRC}/open.rs`]: '// @stub 2.3\n',
  [`${SRC}/pricing/mod.rs`]: '',
  'crates/orders/tests/api.rs': '',
  'crates/orders/tests/api_slow.rs': '',
  'crates/orders/tests/api_perf.rs': '',
  'notes.rs': '',
};

// A project with a workspace, one crate and stub tools. Returns edit(rel, options), which runs
// the hook as if `rel` had just been edited and reports its status, stderr and the tools' calls.
function setup(t, files = FILES) {
  const repo = makeProject(t);
  Object.entries(files).forEach(([rel, body]) => write(repo, rel, body));
  const tools = stubTools(repo, TOOLS);
  const edit = (rel, { agentType = 'implementor', agentId = 'a-1', env = {}, project = repo, base = repo } = {}) => {
    tools.reset();
    const r = runHook(repo, 'check-file.sh', {
      agent_id: agentId, agent_type: agentType, tool_input: { file_path: path.join(base, ...rel.split('/')) },
    }, { env: { ...tools.env, ...env }, project });
    const calls = tools.calls();
    return { status: r.status, stderr: r.stderr, calls, runs: calls.filter((c) => /^(nextest run|test) /.test(c)) };
  };
  return Object.assign(edit, { repo });
}

function clean(r, what) {
  assert.equal(r.status, 0, `${what}: ${r.stderr}`);
  assert.ok(r.calls.includes(CLIPPY), `${what}: the clippy stub was not reached; calls were ${JSON.stringify(r.calls)}`);
}

test('an edited contract runs its own tests and no others', (t) => {
  const edit = setup(t);
  const r = edit(`${SRC}/line_total.rs`);
  clean(r, 'line_total.rs');
  assert.deepEqual(r.runs, ['nextest run -p orders --no-tests=pass line_total::tests::']);
  assert.ok(r.calls.some((c) => /^rustfmt --check --edition 2024 -- .*line_total\.rs$/.test(c)), JSON.stringify(r.calls));
});

test('an edited <name>_tests.rs runs the filter <name>::tests::', (t) => {
  const edit = setup(t);
  const r = edit(`${SRC}/line_total_tests.rs`, { agentType: 'architect' });
  clean(r, 'line_total_tests.rs');
  assert.deepEqual(r.runs, ['nextest run -p orders --no-tests=pass line_total::tests::']);
});

test('an edited tests/<name>.rs runs that test binary', (t) => {
  const edit = setup(t);
  const r = edit('crates/orders/tests/api.rs', { agentType: 'architect' });
  clean(r, 'tests/api.rs');
  assert.deepEqual(r.runs, ['nextest run -p orders --no-tests=pass --test api']);
});

test('a slow or perf test gets rustfmt and clippy and no test run, with nextest or without', (t) => {
  const edit = setup(t);
  const files = [`${SRC}/line_total_slow_tests.rs`, `${SRC}/line_total_perf_tests.rs`, 'crates/orders/tests/api_slow.rs', 'crates/orders/tests/api_perf.rs'];
  for (const file of files) {
    for (const nextest of ['0', '1']) {
      const r = edit(file, { agentType: 'architect', env: { STUB_NEXTEST: nextest } });
      clean(r, file);
      assert.deepEqual(r.runs, [], `${file}, nextest ${nextest === '0' ? 'installed' : 'missing'}`);
    }
  }
});

test('lib.rs, mod.rs, types.rs, a file with a stub, and a test-writer\'s edit run no tests', (t) => {
  const edit = setup(t);
  for (const file of [`${SRC}/lib.rs`, `${SRC}/pricing/mod.rs`, `${SRC}/types.rs`, `${SRC}/open.rs`]) {
    const r = edit(file);
    clean(r, file);
    assert.deepEqual(r.runs, [], file);
  }
  const r = edit(`${SRC}/line_total_tests.rs`, { agentType: 'test-writer' });
  clean(r, 'a test-writer');
  assert.deepEqual(r.runs, [], 'a test-writer runs its own red tests');
});

test('without cargo-nextest the tests run through cargo test, with the same filter', (t) => {
  const edit = setup(t);
  const contract = edit(`${SRC}/line_total.rs`, { env: { STUB_NEXTEST: '1' } });
  clean(contract, 'line_total.rs');
  assert.deepEqual(contract.runs, ['test --quiet -p orders -- line_total::tests::']);
  const api = edit('crates/orders/tests/api.rs', { agentType: 'architect', env: { STUB_NEXTEST: '1' } });
  assert.deepEqual(api.runs, ['test --quiet -p orders --test api']);
});

test('nothing runs for a file that is not Rust, is in no crate, or in a project with no workspace yet', (t) => {
  const edit = setup(t);
  for (const file of ['crates/orders/Cargo.toml', 'notes.rs']) {
    const r = edit(file);
    assert.equal(r.status, 0, file);
    assert.deepEqual(r.calls.filter((c) => !c.startsWith('rustfmt')), [], file);
  }
  const bare = setup(t, { [`${SRC}/line_total.rs`]: '', 'crates/orders/Cargo.toml': '[package]\nname = "orders"\n' });
  const r = bare(`${SRC}/line_total.rs`);
  assert.equal(r.status, 0, r.stderr);
  assert.deepEqual(r.calls, [], 'no root Cargo.toml');
});

const ERROR_HERE = 'crates/orders/src/line_total.rs:3:5: error: mismatched types\n';
const ERROR_ELSEWHERE = 'crates/orders/src/other.rs:9:1: error: cannot find value\n';

test('a sub-agent is shown clippy errors in the file it edited, and not a sibling\'s', (t) => {
  const edit = setup(t);
  const env = { STUB_CLIPPY_EXIT: '101', STUB_CLIPPY_OUT: ERROR_HERE + ERROR_ELSEWHERE };
  const r = edit(`${SRC}/line_total.rs`, { env });
  assert.equal(r.status, 2, 'an error in the edited file must fail the hook');
  assert.match(r.stderr, /== clippy -p orders\n.*line_total\.rs:3:5: error: mismatched types/);
  assert.doesNotMatch(r.stderr, /other\.rs/);
  assert.deepEqual(r.runs, [], 'no tests after a failed lint');

  const sibling = edit(`${SRC}/line_total.rs`, { env: { STUB_CLIPPY_EXIT: '101', STUB_CLIPPY_OUT: ERROR_ELSEWHERE } });
  assert.equal(sibling.status, 0, 'an error only in another file is a sibling mid-write');

  // The main session has no agent id.
  const main = edit(`${SRC}/line_total.rs`, { agentId: '', agentType: 'orchestrator', env: { STUB_CLIPPY_EXIT: '101', STUB_CLIPPY_OUT: ERROR_ELSEWHERE } });
  assert.equal(main.status, 2, 'the main session sees every error');
  assert.match(main.stderr, /other\.rs:9:1/);
});

test('clippy errors printed with backslashes, as cargo prints them on Windows, are still matched', (t) => {
  const edit = setup(t);
  const r = edit(`${SRC}/line_total.rs`, { env: { STUB_CLIPPY_EXIT: '101', STUB_CLIPPY_OUT: ERROR_HERE.replaceAll('/', '\\') } });
  assert.equal(r.status, 2);
  assert.match(r.stderr, /crates\/orders\/src\/line_total\.rs:3:5/);
});

test('the project directory and the file may arrive in different spellings of the same path', (t) => {
  const edit = setup(t);
  const other = otherSpelling(edit.repo);
  if (!other) return t.skip('this machine spells the temp directory one way only');
  const env = { STUB_CLIPPY_EXIT: '101', STUB_CLIPPY_OUT: ERROR_HERE };
  // The file in the long spelling against a project directory in the short one, then the reverse.
  for (const [what, options] of [['file long, project short', { base: other }], ['file short, project long', { project: other }]]) {
    const failing = edit(`${SRC}/line_total.rs`, { ...options, env });
    assert.equal(failing.status, 2, `${what}: the edited file's own error was hidden`);
    assert.match(failing.stderr, /line_total\.rs:3:5: error/, what);
    const passing = edit(`${SRC}/line_total.rs`, options);
    clean(passing, what);
    assert.deepEqual(passing.runs, ['nextest run -p orders --no-tests=pass line_total::tests::'], what);
  }
  return undefined;
});

test('a file rustfmt would change fails the hook, with the command that fixes it', (t) => {
  const edit = setup(t);
  const r = edit(`${SRC}/line_total.rs`, { env: { STUB_RUSTFMT_EXIT: '1' } });
  assert.equal(r.status, 2);
  assert.match(r.stderr, /== rustfmt --check \(run: rustfmt --edition 2024 crates\/orders\/src\/line_total\.rs\)/);
});
