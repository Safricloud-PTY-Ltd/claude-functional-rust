// Tests for git-policy.cjs: which git and gh commands a sub-agent may run. Each case pipes the
// hook's JSON into the policy, as guard-git.sh does, and reads its exit status (0 allows, 2
// blocks) and its reason. The policy only decides; no command here is ever run.
// Run: node --test .claude/hooks/git-policy.test.cjs
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { makeProject, write, runHook } = require('./hook-test-kit.cjs');

const POLICY = path.join(__dirname, 'git-policy.cjs');

// Asks the policy about one command, as the given agent, in the given project.
function ask(agentType, command, { agentId = `a-${agentType}`, project = os.tmpdir() } = {}) {
  return spawnSync(process.execPath, [POLICY], {
    input: JSON.stringify({ agent_id: agentId, agent_type: agentType, cwd: project, tool_input: { command } }),
    env: { ...process.env, CLAUDE_PROJECT_DIR: project },
    encoding: 'utf8',
  });
}
const allowed = (r, what) => assert.equal(r.status, 0, `${what} was refused: ${r.stderr}`);
function refused(r, pattern, what) {
  assert.equal(r.status, 2, `${what} was allowed`);
  assert.match(r.stderr, pattern, what);
}

test('a sub-agent may read with gh: views, lists, help in any form, and a project board', () => {
  for (const command of [
    'gh --help',
    'gh help issue',
    'gh help issue create',
    'gh pr view 3',
    'gh pr checks 3 --watch',
    'gh issue list --label bug --limit 100',
    'gh issue list --help',
    'gh issue --help',
    'gh issue create --help',
    'gh issue create -h',
    'gh pr merge --help',
    'gh project --help',
    'gh project item-add --help',
    'gh auth login --help',
    'gh project list --owner o',
    'gh project view 3 --owner o',
    'gh project field-list 3 --owner o --format json',
    'gh project item-list 3 --owner o --limit 1000 --format json',
    'gh api repos/o/r/issues/3/sub_issues -q .[].number',
    'gh search issues board',
  ]) allowed(ask('architect', command), command);
});

test('a sub-agent may not change GitHub, however the command is dressed', () => {
  const not = /is not allowed here\. Sub-agents run read-only gh/;
  for (const command of [
    'gh issue create --title x',
    'gh issue close 3',
    'gh pr merge 3 --squash',
    'gh label create effort',
    'gh project create --owner o --title t',
    'gh project item-add 3 --owner o --url u',
    'gh project item-edit --id i --project-id p --field-id f --single-select-option-id s',
    'gh project link 3 --owner o --repo o/r',
    // --help here is the title, not a request for help: the flag in front of it takes a value.
    'gh issue create --title --help',
    'gh issue create --title x --help',
    // -h is the hostname, the host and the homepage under these three.
    'gh auth login -h github.com',
    'gh config set -h github.com editor vim',
    'gh repo create x -h https://example.com',
    // An alias or an extension is handed its arguments as they are.
    'gh my-alias --help',
    'gh co --help',
  ]) refused(ask('architect', command), not, command);
  refused(ask('architect', 'gh api -X POST repos/o/r/issues/3/sub_issues -F sub_issue_id=9'), /sub-agents may only read from the API/, 'a POST');
  refused(ask('architect', "gh api graphql -f query='query { viewer { login } }'"), /sub-agents may only (read from the API|make REST GET requests)/, 'graphql');
});

test('the main session, which has no agent id, is not asked', () => {
  const r = spawnSync(process.execPath, [POLICY], {
    input: JSON.stringify({ agent_type: 'orchestrator', tool_input: { command: 'gh pr merge 3 --squash && git push --force' } }),
    encoding: 'utf8',
  });
  assert.equal(r.status, 0, r.stderr);
});

test('every sub-agent may read with git, and none may change the branch, the tree or the remote', () => {
  for (const type of ['architect', 'implementor', 'test-writer', 'designer']) {
    for (const command of ['git status --short', 'git log --oneline -5', 'git --no-pager diff HEAD -- src', 'git branch --show-current', 'cd crates && git grep -n stub']) {
      allowed(ask(type, command), `${type}: ${command}`);
    }
    for (const command of ['git push', 'git switch main', 'git stash', 'git restore .', 'git reset --hard', 'git branch -D x', 'git -c core.hooksPath=x status']) {
      refused(ask(type, command), /guard-git: /, `${type}: ${command}`);
    }
  }
  refused(ask('implementor', "git commit -m 'x' -- crates/orders/src/a.rs"), /`git commit` is not allowed here\. Sub-agents edit; the architect commits/, 'an implementor commit');
});

test('git or gh hidden behind a wrapper, a substitution or a heredoc is refused, and so is unparseable input', () => {
  for (const command of [
    'bash -c "git push"',
    'xargs git add',
    'env GIT_DIR=x git status',
    'echo $(git rev-parse HEAD)',
    'echo `gh pr view 3`',
    "cat <<EOF | gh issue create --body-file -\nEOF",
  ]) refused(ask('architect', command), /guard-git: /, command);
  const bad = spawnSync(process.execPath, [POLICY], { input: 'not json', encoding: 'utf8' });
  assert.equal(bad.status, 2);
  assert.match(bad.stderr, /could not check this command/);
});

test('a command that only mentions git or gh in quoted text, or not at all, passes', () => {
  for (const command of ['echo "run git push later"', 'cargo nextest run -p orders', 'grep -rn "gh pr merge" docs']) {
    allowed(ask('implementor', command), command);
  }
});

// A real repository with one territory: billing holds crates/billing, a-billing is its architect.
function repository(t) {
  const repo = makeProject(t);
  write(repo, 'crates/billing/src/a.rs', 'one\n');
  write(repo, 'crates/billing/src/b.rs', 'one\n');
  write(repo, 'crates/ledger/src/c.rs', 'one\n');
  write(repo, 'Cargo.toml', '[workspace]\n');
  write(repo, '.gitignore', '.claude/territories/\n');
  const git = (...args) => {
    const r = spawnSync('git', ['-c', 'user.name=test', '-c', 'user.email=test@example.invalid', ...args], { cwd: repo, encoding: 'utf8' });
    assert.equal(r.status, 0, `git ${args.join(' ')}: ${r.stderr}`);
  };
  git('init', '-q');
  git('add', '-A');
  git('commit', '-q', '-m', 'start');
  write(repo, '.claude/territories/billing', 'crates/billing/*\nagent a-billing\n');
  ['crates/billing/src/a.rs', 'crates/ledger/src/c.rs', 'Cargo.toml'].forEach((rel) => write(repo, rel, 'two\n'));
  write(repo, 'crates/billing/src/new.rs', 'new\n');
  return repo;
}

test('an architect may add and commit its own files, each named literally', (t) => {
  const project = repository(t);
  const architect = (command) => ask('architect', command, { agentId: 'a-billing', project });
  for (const command of [
    'git add -- crates/billing/src/a.rs',
    'git add -- crates/billing/src/a.rs crates/billing/src/new.rs',
    "git commit -m 'billing: a' -- crates/billing/src/a.rs",
    'git commit -q -m "billing: a" -- crates/billing/src/a.rs',
  ]) allowed(architect(command), command);
});

test('an architect may not add or commit outside its territory, or by anything but a literal path', (t) => {
  const project = repository(t);
  const architect = (command) => ask('architect', command, { agentId: 'a-billing', project });
  refused(architect('git add -- crates/ledger/src/c.rs'), /would take in files outside your territory/, 'a sibling\'s file');
  refused(architect("git commit -m 'x' -- Cargo.toml"), /would take in files outside your territory/, 'a reserve file');
  refused(architect('git add -- crates/billing'), /names a directory/, 'a directory');
  refused(architect("git add -- 'crates/billing/*.rs'"), /git reads that as a pattern/, 'a pathspec pattern');
  refused(architect('git add -- crates/billing/src/*.rs'), /a word bash rewrites/, 'a glob');
  refused(architect('git commit -m "$MSG" -- crates/billing/src/a.rs'), /a word bash rewrites/, 'a variable in the message');
  refused(architect('git add -A'), /takes explicit paths and no -A/, '-A');
  refused(architect("git commit -am 'x'"), /takes `-m <msg> -- <paths>` and no -am/, '-a');
  refused(architect("git commit --amend -m 'x' -- crates/billing/src/a.rs"), /and no --amend/, '--amend');
  refused(architect("git commit -m 'x'"), /needs at least one path/, 'no paths');
  refused(architect('git push'), /An architect runs read-only git, plus/, 'a push');
  refused(ask('architect', 'git add -- crates/billing/src/a.rs', { agentId: 'a-unregistered', project }), /no territory names architect a-unregistered/, 'an unregistered architect');
});

test('guard-git.sh passes the policy\'s verdict through, and blocks when the policy cannot run', (t) => {
  const repo = makeProject(t);
  const input = (command) => ({ agent_id: 'a-1', agent_type: 'implementor', cwd: repo, tool_input: { command } });
  assert.equal(runHook(repo, 'guard-git.sh', input('git status')).status, 0);
  const push = runHook(repo, 'guard-git.sh', input('git push'));
  assert.equal(push.status, 2);
  assert.match(push.stderr, /`git push` is not allowed here/);
  fs.rmSync(path.join(repo, '.claude', 'hooks', 'git-policy.cjs'));
  const broken = runHook(repo, 'guard-git.sh', input('git status'));
  assert.equal(broken.status, 2);
  assert.match(broken.stderr, /the git policy failed to run/);
});
