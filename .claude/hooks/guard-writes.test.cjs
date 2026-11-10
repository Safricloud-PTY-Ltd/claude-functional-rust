// Tests for guard-writes.sh: who may write which file. Each test runs the hook itself, from a
// throwaway project, on the input shape Claude Code sends: an absolute file_path in the
// platform's own spelling (backslashes on Windows). Run: node --test .claude/hooks/guard-writes.test.cjs
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { makeProject, write, otherSpelling, runHook } = require('./hook-test-kit.cjs');

const STUB = '// @stub 2.3\n';

// A project holding the given files (forward-slash path -> content). Returns guard(type, rel),
// which asks the hook about a path inside the project; guard.raw takes the path exactly as given.
function setup(t, files = {}) {
  const repo = makeProject(t);
  Object.entries(files).forEach(([rel, body]) => write(repo, rel, body));
  const raw = (agentType, file, agentId = `a-${agentType}`) => runHook(repo, 'guard-writes.sh', {
    agent_id: agentId, agent_type: agentType, tool_input: { file_path: file },
  });
  const guard = (agentType, rel, agentId) => raw(agentType, path.join(repo, ...rel.split('/')), agentId);
  return Object.assign(guard, { raw, repo });
}

const allowed = (r, what) => assert.equal(r.status, 0, `${what} was refused: ${r.stderr}`);
function refused(r, pattern, what) {
  assert.equal(r.status, 2, `${what} was allowed`);
  assert.match(r.stderr, pattern, what);
}

const ORDERS = {
  'crates/orders/Cargo.toml': '[package]\nname = "orders"\n',
  'crates/orders/src/lib.rs': '//! orders\n',
  'crates/orders/src/line_total.rs': STUB,
  'crates/orders/src/pricing/round_minor.rs': STUB,
};

test('a test-writer may write the tests beside a contract, and tests/*.rs in a crate with a lib.rs', (t) => {
  const guard = setup(t, { ...ORDERS, 'xtask/src/core/check_steps.rs': STUB, 'xtask/src/lib.rs': '' });
  for (const rel of [
    'crates/orders/src/line_total_tests.rs',
    'crates/orders/src/pricing/round_minor_tests.rs',
    'crates/orders/tests/api.rs',
    'xtask/src/core/check_steps_tests.rs',
    'xtask/tests/api.rs',
  ]) allowed(guard('test-writer', rel), rel);
});

test('a test-writer may write a slow or perf test beside the same contract, and as tests/<name>_slow.rs', (t) => {
  const guard = setup(t, ORDERS);
  for (const rel of [
    'crates/orders/src/line_total_slow_tests.rs',
    'crates/orders/src/line_total_perf_tests.rs',
    'crates/orders/src/pricing/round_minor_slow_tests.rs',
    'crates/orders/tests/api_slow.rs',
    'crates/orders/tests/api_perf.rs',
  ]) allowed(guard('test-writer', rel), rel);
});

test('a test-writer is refused tests with no contract, and told which contract was looked for', (t) => {
  const guard = setup(t, ORDERS);
  const at = 'crates/orders/src';
  for (const name of ['missing_tests.rs', 'missing_slow_tests.rs', 'missing_perf_tests.rs']) {
    refused(guard('test-writer', `${at}/${name}`), /no contract at crates\/orders\/src\/missing\.rs for /, name);
  }
});

test('a test-writer is refused everything that is not a test file in a crate', (t) => {
  const guard = setup(t, { ...ORDERS, 'crates/bare/src/main.rs': '' });
  refused(guard('test-writer', 'crates/orders/src/line_total.rs'), /test-writer writes only <name>_tests\.rs/, 'a contract');
  refused(guard('test-writer', 'crates/orders/src/lib.rs'), /test-writer writes only <name>_tests\.rs/, 'lib.rs');
  refused(guard('test-writer', 'crates/orders/tests/fixtures/sample.rs'), /not files below it/, 'a file below tests/');
  refused(guard('test-writer', 'crates/bare/tests/api.rs'), /no src\/lib\.rs in crates\/bare/, 'tests/ in a crate with no lib.rs');
  refused(guard('test-writer', 'crates/orders/Cargo.toml'), /may only write \.rs files under a crate's src\/ or tests\//, 'Cargo.toml');
  refused(guard('test-writer', 'README.md'), /may only write \.rs files under a crate's src\/ or tests\//, 'a file at the root');
  refused(guard.raw('test-writer', path.join(os.tmpdir(), 'elsewhere_tests.rs')), /may only write inside the repository/, 'a file outside the project');
});

test('an implementor edits an existing file that carries a stub, and nothing else', (t) => {
  const guard = setup(t, {
    ...ORDERS,
    'crates/orders/src/done.rs': 'pub fn done() {}\n',
    'crates/orders/src/line_total_tests.rs': '',
    'crates/orders/src/line_total_slow_tests.rs': '',
    'crates/orders/src/line_total_perf_tests.rs': '',
    'crates/orders/tests/api.rs': '',
    'crates/orders/tests/api_slow.rs': '',
  });
  allowed(guard('implementor', 'crates/orders/src/line_total.rs'), 'a file with a stub');
  refused(guard('implementor', 'crates/orders/src/done.rs'), /has no @stub marker/, 'an implemented file');
  refused(guard('implementor', 'crates/orders/src/brand_new.rs'), /implementor does not create files/, 'a new file');
  for (const rel of [
    'crates/orders/src/line_total_tests.rs',
    'crates/orders/src/line_total_slow_tests.rs',
    'crates/orders/src/line_total_perf_tests.rs',
    'crates/orders/tests/api.rs',
    'crates/orders/tests/api_slow.rs',
  ]) refused(guard('implementor', rel), /implementor does not edit tests/, rel);
  refused(guard('implementor', 'crates/orders/Cargo.toml'), /may only write \.rs files/, 'Cargo.toml');
});

test('a file has one owner per phase: the first sub-agent to write it holds it', (t) => {
  const guard = setup(t, ORDERS);
  const rel = 'crates/orders/src/line_total.rs';
  allowed(guard('implementor', rel, 'a-first'), 'the first implementor');
  assert.equal(fs.readFileSync(path.join(guard.repo, '.claude', 'locks', 'crates__orders__src__line_total.rs'), 'utf8'), 'a-first');
  refused(guard('implementor', rel, 'a-second'), /is owned by another agent this phase/, 'a second implementor');
  // The holder keeps the file after its own edit removed the marker.
  write(guard.repo, rel, 'pub fn line_total() {}\n');
  allowed(guard('implementor', rel, 'a-first'), 'the holder, once the stub is gone');
  const tests = 'crates/orders/src/line_total_tests.rs';
  allowed(guard('test-writer', tests, 'a-writer'), 'the first test-writer');
  refused(guard('test-writer', tests, 'a-third'), /is owned by another agent this phase/, 'a second test-writer on a held test file');
});

test('the main session, and an agent type the guard does not know, are not fenced', (t) => {
  const guard = setup(t, ORDERS);
  const anywhere = path.join(guard.repo, 'Cargo.toml');
  const main = runHook(guard.repo, 'guard-writes.sh', { agent_type: 'orchestrator', tool_input: { file_path: anywhere } });
  allowed(main, 'the main session, which has no agent_id');
  // Explore and any other type follow Claude Code's own permission flow. Named here so a new
  // agent that should be fenced gets its own arm, as the designer has.
  allowed(guard.raw('Explore', anywhere), 'Explore');
  allowed(guard.raw('some-new-agent', anywhere), 'an unknown agent type');
});

test('input the guard cannot parse blocks the write', (t) => {
  const guard = setup(t);
  const r = runHook(guard.repo, 'guard-writes.sh', 'not json');
  refused(r, /could not parse hook input; write blocked/, 'unparseable input');
});

test('an architect is held to the territory that names it, and to its scratch folder outside the project', (t) => {
  const guard = setup(t, ORDERS);
  const parent = fs.mkdtempSync(path.join(os.tmpdir(), 'hook-test-scratch-'));
  t.after(() => fs.rmSync(parent, { recursive: true, force: true }));
  const scratch = path.join(parent, 'orders');
  fs.mkdirSync(scratch);
  write(guard.repo, '.claude/territories/orders', `crates/orders/*\nscratch ${scratch.split(path.sep).join('/')}\nagent a-orders\n`);
  allowed(guard('architect', 'crates/orders/src/brand_new.rs', 'a-orders'), 'a new file in its territory');
  allowed(guard('architect', 'crates/orders/Cargo.toml', 'a-orders'), 'its manifest');
  allowed(guard.raw('architect', path.join(scratch, 'codex', 'T-orders-1.1.brief'), 'a-orders'), 'its scratch folder');
  refused(guard('architect', 'Cargo.toml', 'a-orders'), /outside your territory, orders/, 'a reserve file');
  refused(guard.raw('architect', path.join(parent, 'elsewhere.md'), 'a-orders'), /outside your scratch folder/, 'beside its scratch folder');
  refused(guard('architect', 'crates/orders/src/lib.rs', 'a-unregistered'), /no territory names architect a-unregistered/, 'an architect no territory names');
});

test('with territories live, a test-writer or implementor writes only inside one', (t) => {
  const guard = setup(t, { ...ORDERS, 'crates/ledger/src/lib.rs': '', 'crates/ledger/src/post.rs': STUB });
  write(guard.repo, '.claude/territories/orders', 'crates/orders/*\nagent a-orders\n');
  allowed(guard('implementor', 'crates/orders/src/line_total.rs'), 'inside a territory');
  refused(guard('implementor', 'crates/ledger/src/post.rs'), /is in no architect's territory/, 'a stub in no territory');
  refused(guard('test-writer', 'crates/ledger/src/post_tests.rs'), /is in no architect's territory/, 'a test in no territory');
});

test('a path is judged after .. is resolved, so it cannot climb to a file the agent may not write', (t) => {
  const guard = setup(t, { ...ORDERS, 'crates/orders/src/done.rs': 'pub fn done() {}\n' });
  // Raw strings: path.join would resolve the .. before the hook ever saw it.
  const src = [guard.repo, 'crates', 'orders', 'src'].join(path.sep);
  refused(guard.raw('implementor', [src, 'pricing', '..', 'done.rs'].join(path.sep)), /crates\/orders\/src\/done\.rs has no @stub marker/, 'an implemented file reached through ..');
  refused(guard.raw('test-writer', [src, '..', '..', '..', 'README_tests.rs'].join(path.sep)), /may only write \.rs files under a crate's src\/ or tests\//, 'a climb to the root');
  allowed(guard.raw('implementor', [src, 'pricing', '..', 'line_total.rs'].join(path.sep)), 'a stub reached through ..');
});

test('the project directory and the file may arrive in different spellings of the same path', (t) => {
  const guard = setup(t, ORDERS);
  const other = otherSpelling(guard.repo);
  if (!other) return t.skip('this machine spells the temp directory one way only');
  allowed(guard.raw('implementor', path.join(other, 'crates', 'orders', 'src', 'line_total.rs')), 'a stub, long spelling');
  refused(guard.raw('test-writer', path.join(other, 'crates', 'orders', 'src', 'missing_tests.rs')), /no contract at crates\/orders\/src\/missing\.rs/, 'a missing contract, long spelling');
  return undefined;
});

const DESIGN = 'contributions/in-progress/2026-10-01-invoicing/design';

test('a designer may write inside its effort design folder, whether or not the file exists yet', (t) => {
  // design.md exists on disk; the others don't yet.
  const guard = setup(t, { [`${DESIGN}/design.md`]: '' });
  for (const file of ['design.md', 'option-a.html', 'assets/logo.svg']) allowed(guard('designer', `${DESIGN}/${file}`), file);
});

test('a designer is refused everywhere else, product code and the rest of its effort folder included', (t) => {
  const guard = setup(t, ORDERS);
  const only = /the designer writes only inside contributions\/in-progress\/<id>\/design\//;
  for (const rel of [
    'crates/orders/src/line_total.rs',
    'crates/web/src/theme/tokens.rs',
    'contributions/in-progress/2026-10-01-invoicing/review.md',
    'contributions/in-progress/2026-10-01-invoicing/billing/plan.md',
    'contributions/complete/2026-09-05-order-totals/design/design.md',
    'contributions/in-progress/design/design.md',
    '.claude/agents/designer.md',
    'CLAUDE.md',
  ]) refused(guard('designer', rel), only, rel);
  refused(guard.raw('designer', path.join(os.tmpdir(), 'outside-the-project.html')), only, 'a file outside the project');
});

test('a designer cannot climb out of its folder with .. in the path it gives', (t) => {
  const guard = setup(t);
  // Raw strings: path.join would resolve the .. before the hook ever saw it.
  const design = [guard.repo, ...DESIGN.split('/')].join(path.sep);
  for (const segments of [
    [design, '..', 'review.md'],
    [design, '..', '..', '..', '..', 'CLAUDE.md'],
    [guard.repo, '..', 'outside-the-project.html'],
  ]) refused(guard.raw('designer', segments.join(path.sep)), /the designer writes only inside/, segments.join(path.sep));
  // The same shape that stays inside is still allowed.
  allowed(guard.raw('designer', [design, 'drafts', '..', 'option-a.html'].join(path.sep)), 'a .. that stays inside');
});
