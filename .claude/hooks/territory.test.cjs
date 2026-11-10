// Tests for territory.cjs: the canonical repo-relative path, territory globs, the scratch line,
// and who is refused what. Run: node --test .claude/hooks/territory.test.cjs
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { makeProject, write, otherSpelling } = require('./hook-test-kit.cjs');
const { fromShell, repoRelative, load, holders, refusal } = require('./territory.cjs');

const fwd = (p) => p.split(path.sep).join('/');

// A scratch folder outside the project, named after the territory as the guard requires.
function scratchFor(t, name) {
  const parent = fs.mkdtempSync(path.join(os.tmpdir(), 'hook-test-scratch-'));
  t.after(() => fs.rmSync(parent, { recursive: true, force: true }));
  const dir = path.join(parent, name);
  fs.mkdirSync(dir);
  return dir;
}

test('repoRelative gives a forward-slash path inside the project, for a file that exists and one that does not', (t) => {
  const repo = makeProject(t);
  write(repo, 'crates/orders/src/line_total.rs');
  assert.equal(repoRelative(repo, path.join(repo, 'crates', 'orders', 'src', 'line_total.rs')), 'crates/orders/src/line_total.rs');
  assert.equal(repoRelative(repo, path.join(repo, 'crates', 'orders', 'src', 'not_yet.rs')), 'crates/orders/src/not_yet.rs');
  assert.equal(repoRelative(repo, path.join(repo, 'crates', 'new_crate', 'src', 'lib.rs')), 'crates/new_crate/src/lib.rs');
  assert.equal(repoRelative(repo, 'crates/orders/src/line_total.rs'), 'crates/orders/src/line_total.rs');
});

test('repoRelative resolves .. before it answers, and answers null outside the project', (t) => {
  const repo = makeProject(t);
  write(repo, 'crates/orders/src/line_total.rs');
  // Raw strings: path.join would resolve the .. before repoRelative ever saw it.
  const inside = [repo, 'crates', 'orders', '..', 'orders', 'src', 'line_total.rs'].join(path.sep);
  assert.equal(repoRelative(repo, inside), 'crates/orders/src/line_total.rs');
  assert.equal(repoRelative(repo, [repo, 'crates', '..', '..', 'elsewhere.rs'].join(path.sep)), null);
  assert.equal(repoRelative(repo, [repo, '..', 'elsewhere.rs'].join(path.sep)), null);
  assert.equal(repoRelative(repo, path.join(os.tmpdir(), 'elsewhere.rs')), null);
});

test('repoRelative reads the same file through either spelling of the project directory', (t) => {
  const repo = makeProject(t);
  const other = otherSpelling(repo);
  if (!other) return t.skip('this machine spells the temp directory one way only');
  write(repo, 'crates/orders/src/line_total.rs');
  const rel = 'crates/orders/src/line_total.rs';
  assert.equal(repoRelative(repo, path.join(other, ...rel.split('/'))), rel);
  assert.equal(repoRelative(other, path.join(repo, ...rel.split('/'))), rel);
  assert.equal(repoRelative(repo, path.join(other, 'crates', 'orders', 'src', 'not_yet.rs')), 'crates/orders/src/not_yet.rs');
  return undefined;
});

test('fromShell turns a Git Bash drive path back into a Windows one, and leaves others alone', { skip: process.platform !== 'win32' }, () => {
  assert.equal(fromShell('/c/Users/someone/repo'), 'c:/Users/someone/repo');
  assert.equal(fromShell('/c'), 'c:/');
  assert.equal(fromShell('C:\\Users\\someone'), 'C:\\Users\\someone');
  assert.equal(fromShell('crates/orders'), 'crates/orders');
});

test('a territory file gives globs whose * crosses /, agent lines, and ignores comments, blanks and CRLF', (t) => {
  const repo = makeProject(t);
  write(repo, '.claude/territories/billing', '# billing\r\n\r\ncrates/billing/*\r\ncontributions/in-progress/x/billing/*\r\nagent a-1\r\nagent a-2\r\n');
  write(repo, '.claude/territories/ledger', 'crates/ledger/src/tax/*\nagent a-3\n');
  const territories = load(repo);
  const names = (rel) => holders(territories, rel).map((x) => x.name);
  assert.deepEqual(territories.find((x) => x.name === 'billing').agents, ['a-1', 'a-2']);
  assert.deepEqual(names('crates/billing/src/pricing/line_total.rs'), ['billing']);
  assert.deepEqual(names('contributions/in-progress/x/billing/plan.md'), ['billing']);
  assert.deepEqual(names('crates/ledger/src/tax/vat/rate.rs'), ['ledger']);
  assert.deepEqual(names('crates/ledger/src/post.rs'), []);
  assert.deepEqual(names('crates/billing-rules/src/lib.rs'), []);
  assert.deepEqual(names('Cargo.toml'), []);
});

test('with no territories directory there are no territories, and a sub-agent is not fenced by them', (t) => {
  const repo = makeProject(t);
  assert.deepEqual(load(repo), []);
  assert.equal(refusal([], 'implementor', 'a-1', 'crates/orders/src/line_total.rs'), null);
  assert.equal(refusal([], 'test-writer', 'a-1', 'crates/orders/src/line_total_tests.rs'), null);
});

test('an architect writes inside the territory that names it, and nowhere else in the project', (t) => {
  const repo = makeProject(t);
  write(repo, '.claude/territories/billing', 'crates/billing/*\nagent a-billing\n');
  write(repo, '.claude/territories/ledger', 'crates/ledger/*\nagent a-ledger\n');
  const territories = load(repo);
  assert.equal(refusal(territories, 'architect', 'a-billing', 'crates/billing/src/lib.rs'), null);
  assert.match(refusal(territories, 'architect', 'a-billing', 'crates/ledger/src/lib.rs'), /outside your territory, billing\. .*CROSS report/);
  assert.match(refusal(territories, 'architect', 'a-billing', 'Cargo.toml'), /outside your territory, billing\. A shared file is a NEEDS report/);
  assert.match(refusal(territories, 'architect', 'a-nobody', 'crates/billing/src/lib.rs'), /no territory names architect a-nobody/);
});

test('a file two territories both claim is refused to everyone, naming both', (t) => {
  const repo = makeProject(t);
  write(repo, '.claude/territories/billing', 'crates/billing/*\nagent a-billing\n');
  write(repo, '.claude/territories/tax', 'crates/billing/src/tax/*\nagent a-tax\n');
  const territories = load(repo);
  const rel = 'crates/billing/src/tax/rate.rs';
  for (const [type, id] of [['architect', 'a-billing'], ['implementor', 'a-9'], ['test-writer', 'a-9']]) {
    assert.match(refusal(territories, type, id, rel), /is in territories billing and tax\. Territories overlap/, type);
  }
});

test('once any territory exists, a test-writer or an implementor writes only inside one', (t) => {
  const repo = makeProject(t);
  write(repo, '.claude/territories/billing', 'crates/billing/*\nagent a-billing\n');
  const territories = load(repo);
  for (const type of ['implementor', 'test-writer']) {
    assert.equal(refusal(territories, type, 'a-9', 'crates/billing/src/line_total.rs'), null, type);
    assert.match(refusal(territories, type, 'a-9', 'crates/shared/src/stub.rs'), /is in no architect's territory/, type);
    assert.match(refusal(territories, type, 'a-9', null), /is in no architect's territory/, type);
  }
});

test('an agent type the territories do not know is never refused by them', (t) => {
  const repo = makeProject(t);
  write(repo, '.claude/territories/billing', 'crates/billing/*\nagent a-billing\n');
  const territories = load(repo);
  for (const type of ['designer', 'Explore', '']) {
    assert.equal(refusal(territories, type, 'a-9', 'Cargo.toml'), null, type);
    assert.equal(refusal(territories, type, 'a-9', null), null, type);
  }
});

test('an architect may write inside its scratch folder outside the project, and nowhere else outside', (t) => {
  const repo = makeProject(t);
  const scratch = scratchFor(t, 'billing');
  write(repo, '.claude/territories/billing', `crates/billing/*\nscratch ${fwd(scratch)}\nagent a-billing\n`);
  const territories = load(repo);
  const real = fs.realpathSync.native(scratch);
  assert.equal(refusal(territories, 'architect', 'a-billing', null, path.join(real, 'codex', 'T-billing-1.1.brief')), null);
  assert.match(refusal(territories, 'architect', 'a-billing', null, real), /outside your scratch folder/);
  assert.match(refusal(territories, 'architect', 'a-billing', null, path.join(path.dirname(real), 'other', 'x.md')), /outside your scratch folder/);
  assert.match(refusal(territories, 'architect', 'a-billing', null, path.join(`${real}-notes`, 'x.md')), /outside your scratch folder/);
});

test('a scratch line that is relative, misnamed, or inside the project opens nothing, and says why', (t) => {
  const repo = makeProject(t);
  const misnamed = scratchFor(t, 'ledger');
  write(repo, '.claude/territories/none', 'crates/none/*\nagent a-none\n');
  write(repo, '.claude/territories/relative', 'crates/relative/*\nscratch scratchpad/relative\nagent a-relative\n');
  write(repo, '.claude/territories/billing', `crates/billing/*\nscratch ${fwd(misnamed)}\nagent a-billing\n`);
  write(repo, '.claude/territories/inner', `crates/inner/*\nscratch ${fwd(path.join(repo, 'inner'))}\nagent a-inner\n`);
  const territories = load(repo);
  const outside = path.join(fs.realpathSync.native(misnamed), 'x.md');
  assert.match(refusal(territories, 'architect', 'a-none', null, outside), /has no scratch line/);
  assert.match(refusal(territories, 'architect', 'a-relative', null, outside), /is not an absolute path/);
  assert.match(refusal(territories, 'architect', 'a-billing', null, outside), /is not named after the territory, billing/);
  assert.match(refusal(territories, 'architect', 'a-inner', null, outside), /is not outside the repository/);
});

test('the CLI prints the canonical path and then ok or the refusal, which is what the write guard reads', (t) => {
  const repo = makeProject(t);
  write(repo, '.claude/territories/billing', 'crates/billing/*\nagent a-billing\n');
  const cli = (id, type, file) => spawnSync(process.execPath, [path.join(repo, '.claude', 'hooks', 'territory.cjs'), id, type, file], {
    // A fixed cwd the path can't resolve from, wherever node --test was launched.
    cwd: __dirname,
    env: { ...process.env, CLAUDE_PROJECT_DIR: repo },
    encoding: 'utf8',
  }).stdout.split('\n');
  assert.deepEqual(cli('a-billing', 'architect', path.join(repo, 'crates', 'billing', 'src', 'lib.rs')).slice(0, 2), ['crates/billing/src/lib.rs', 'ok']);
  const [rel, why] = cli('a-billing', 'architect', path.join(repo, 'Cargo.toml'));
  assert.equal(rel, 'Cargo.toml');
  assert.match(why, /outside your territory, billing/);
  assert.deepEqual(cli('a-9', 'designer', path.join(os.tmpdir(), 'elsewhere.html')).slice(0, 2), ['', 'ok']);
});
