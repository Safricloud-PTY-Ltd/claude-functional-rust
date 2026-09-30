#!/usr/bin/env node
// Which git commands a sub-agent may run. Called by guard-git.sh with the hook's JSON on stdin;
// exit 0 allows, exit 2 blocks with the reason on stderr. Deny by default:
//
//   main session (no agent_id)  anything
//   every sub-agent             read-only verbs (READ_ONLY below), `git -C <dir>` and
//                               `--no-pager` only as global options
//   architect (a sub-agent)     also `git add` and `git commit`, with explicit paths, when
//                               every file git reports the command would stage or commit lies
//                               in the architect's own territory and in no other
//
// git must run as a plain command: behind xargs, env, a shell -c string, eval, or a command
// substitution the guard can't read it, so it refuses. It fences mistakes by cooperative
// agents; it is not a sandbox.
'use strict';
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const { fromShell, repoRelative, load, refusal } = require('./territory.js');

const READ_ONLY = new Set(['status', 'diff', 'log', 'show', 'grep', 'blame', 'rev-parse', 'ls-files',
  'ls-tree', 'cat-file', 'describe', 'shortlog', 'merge-base', 'archive', 'rev-list', 'show-ref',
  'for-each-ref', 'name-rev', 'check-ignore']);
const LISTING = { branch: ['--show-current', '--list', '-l', '-a', '-r', '-v', '-vv', '--all'], remote: ['-v'] };
const WRAPPERS = new Set(['sh', 'bash', 'zsh', 'dash', 'eval', 'exec', 'xargs', 'env', 'command',
  'sudo', 'nice', 'nohup', 'time', 'timeout', 'pwsh', 'powershell', 'cmd', 'cmd.exe', 'find', 'parallel']);

const isGit = (w) => /(^|[\\/])git(\.exe)?$/i.test(w);

class Refused extends Error {}
const refuse = (why) => { throw new Refused(why); };

// Shell words and operators. Quotes are honoured; redirections are dropped with their target.
function tokenize(s) {
  const out = [];
  let cur = null;
  let quoted = false;
  const push = () => { if (cur !== null) out.push({ word: cur, quoted }); cur = null; quoted = false; };
  for (let i = 0; i < s.length;) {
    const c = s[i];
    if (c === "'") {
      const j = s.indexOf("'", i + 1);
      if (j < 0) refuse('an unterminated quote');
      cur = (cur ?? '') + s.slice(i + 1, j); quoted = true; i = j + 1; continue;
    }
    if (c === '"') {
      let j = i + 1;
      let buf = '';
      while (j < s.length && s[j] !== '"') {
        if (s[j] === '\\' && j + 1 < s.length && '"\\$`\n'.includes(s[j + 1])) { buf += s[j + 1]; j += 2; continue; }
        if (s[j] === '`' || s.startsWith('$(', j)) refuse('a command substitution');
        buf += s[j]; j += 1;
      }
      if (j >= s.length) refuse('an unterminated quote');
      cur = (cur ?? '') + buf; quoted = true; i = j + 1; continue;
    }
    if (c === '\\' && i + 1 < s.length) { cur = (cur ?? '') + s[i + 1]; i += 2; continue; }
    if (c === '`' || s.startsWith('$(', i)) refuse('a command substitution');
    if (s.startsWith('<<', i)) refuse('a heredoc');
    if (c === '#' && cur === null) { const j = s.indexOf('\n', i); i = j < 0 ? s.length : j; continue; }
    if (c === '\n') { push(); out.push({ op: ';' }); i += 1; continue; }
    if (/\s/.test(c)) { push(); i += 1; continue; }
    const redirect = cur === null || /^\d+$/.test(cur) ? /^(&>>?|>>?&?|<&?)(\d+|-)?/.exec(s.slice(i)) : null;
    if (redirect) {
      cur = null; quoted = false;
      out.push({ op: redirect[2] ? 'dup' : 'redirect' });
      i += redirect[0].length; continue;
    }
    const op = ['&&', '||', ';', '|', '&', '(', ')'].find((o) => s.startsWith(o, i));
    if (op) { push(); out.push({ op }); i += op.length; continue; }
    cur = (cur ?? '') + c; i += 1;
  }
  push();
  return out;
}

// Every git invocation in the command: { dir, globals, verb, args, assigned }.
function invocations(command, cwd) {
  const tokens = tokenize(command);
  const segments = [[]];
  for (let k = 0; k < tokens.length; k += 1) {
    const t = tokens[k];
    if (t.op === 'redirect') { k += 1; continue; }        // drop the redirection's target
    if (t.op === 'dup') continue;
    if (t.op) { segments.push([]); continue; }
    segments[segments.length - 1].push(t);
  }
  let dir = cwd;
  const found = [];
  for (const seg of segments) {
    let k = 0;
    while (k < seg.length && !seg[k].quoted && /^[A-Za-z_][A-Za-z0-9_]*=/.test(seg[k].word)) k += 1;
    const assigned = k > 0;
    if (k >= seg.length) continue;
    const head = seg[k].word;
    if (head === 'cd') {
      const to = seg[k + 1]?.word;
      if (to && to !== '-') dir = path.resolve(fromShell(dir), fromShell(to));
      continue;
    }
    if (!isGit(head)) {
      const rest = seg.slice(k + 1);
      if (rest.some((t) => !t.quoted && isGit(t.word))) refuse(`git behind \`${head}\` (if the word is only text, quote it)`);
      if (WRAPPERS.has(path.basename(head).toLowerCase()) && rest.some((t) => /\bgit\b/.test(t.word))) refuse(`git behind \`${head}\``);
      continue;
    }
    let at = dir;
    const globals = [];
    let j = k + 1;
    while (j < seg.length && seg[j].word.startsWith('-')) {
      if (seg[j].word === '-C') { at = path.resolve(fromShell(at), fromShell(seg[j + 1]?.word ?? '.')); j += 2; continue; }
      globals.push(seg[j].word); j += 1;
      if (seg[j - 1].word === '-c') { globals.push(seg[j]?.word ?? ''); j += 1; }
    }
    found.push({ dir: at, globals, verb: seg[j]?.word ?? '', args: seg.slice(j + 1).map((t) => t.word), assigned });
  }
  return found;
}

// Split args into options and paths; `--` ends the options.
function split(args, takesValue) {
  const options = [];
  const paths = [];
  for (let k = 0; k < args.length; k += 1) {
    const a = args[k];
    if (a === '--') { paths.push(...args.slice(k + 1)); break; }
    if (a.startsWith('-') && a !== '-') {
      options.push(a);
      if (takesValue(a)) k += 1;
    } else paths.push(a);
  }
  return { options, paths };
}

function gitLines(dir, args) {
  const r = spawnSync('git', args, { cwd: dir, encoding: 'utf8' });
  if (r.status !== 0) refuse(`the guard could not ask git which files \`git ${args.join(' ')}\` touches (${(r.stderr || '').trim().split('\n')[0]})`);
  return r.stdout.split('\n').map((l) => l.trim()).filter(Boolean);
}

function checkAffected(root, territories, agentId, verb, files) {
  const problems = [...new Set(files)]
    .map((f) => refusal(territories, 'architect', agentId, repoRelative(root, f)))
    .filter(Boolean);
  if (problems.length) {
    refuse(`\`git ${verb}\` would take in files outside your territory:\n  ${problems.slice(0, 5).join('\n  ')}${problems.length > 5 ? `\n  (and ${problems.length - 5} more)` : ''}\nName only your own files.`);
  }
}

function checkAdd(inv, root, territories, agentId) {
  const { options, paths } = split(inv.args, () => false);
  const bad = options.filter((o) => !['-v', '--verbose'].includes(o));
  if (bad.length) refuse(`\`git add\` takes explicit paths and no ${bad.join(' ')}: -A, -u, -f, -p and the rest reach past your own files.`);
  if (paths.length === 0) refuse('`git add` needs at least one path.');
  const staged = gitLines(inv.dir, ['diff', '--name-only', 'HEAD', '--', ...paths]);
  const untracked = gitLines(inv.dir, ['ls-files', '--full-name', '--others', '--exclude-standard', '--', ...paths]);
  checkAffected(root, territories, agentId, 'add', [...staged, ...untracked].map((f) => path.join(root, f)));
}

function checkCommit(inv, root, territories, agentId) {
  const takesValue = (o) => o === '-m' || o === '--message' || /^-[qsvo]+m$/.test(o);
  const { options, paths } = split(inv.args, takesValue);
  const allowed = (o) => ['-q', '--quiet', '-s', '--signoff', '-v', '--verbose', '-o', '--only', '-m', '--message'].includes(o)
    || o.startsWith('--message=') || /^-[qsvo]*m.*$/.test(o) || /^-[qsvo]+$/.test(o);
  const bad = options.filter((o) => !allowed(o));
  if (bad.length) refuse(`\`git commit\` takes \`-m <msg> -- <paths>\` and no ${bad.join(' ')}: -a, -i, --amend, --fixup, -C and the rest commit more than the files you name, or rewrite history.`);
  if (paths.length === 0) refuse('`git commit` needs at least one path; with none it commits the whole shared index, other architects\' staged files included.');
  const files = gitLines(inv.dir, ['diff', '--name-only', 'HEAD', '--', ...paths]);
  checkAffected(root, territories, agentId, 'commit', files.map((f) => path.join(root, f)));
}

function decide(input) {
  const agentId = input.agent_id;
  if (!agentId) return;
  const command = input.tool_input?.command ?? '';
  if (!/\bgit\b/.test(command)) return;
  const agentType = input.agent_type ?? '';
  const root = process.env.CLAUDE_PROJECT_DIR || input.cwd || process.cwd();
  const cwd = input.cwd || root;
  const architect = agentType === 'architect';
  const allowedHere = architect
    ? 'An architect runs read-only git, plus `git add -- <paths>` and `git commit -m <msg> -- <paths>` on its own files. The branch, the working tree and the remote are the orchestrator\'s; ask it in a report.'
    : 'Sub-agents edit; the architect commits. Read-only git (status, diff, log, show, grep, blame) is fine. Put what you changed in your report.';
  let found;
  try {
    found = invocations(command, cwd);
  } catch (e) {
    if (e instanceof Refused) refuse(`${e.message} in a command that mentions git: run git as a plain command so the guard can read it. ${allowedHere}`);
    throw e;
  }
  for (const inv of found) {
    const extra = inv.globals.filter((g) => !['--no-pager'].includes(g));
    if (extra.length) refuse(`\`git ${extra.join(' ')}\`: only -C and --no-pager are allowed before the verb. ${allowedHere}`);
    if (READ_ONLY.has(inv.verb)) continue;
    if (LISTING[inv.verb] && inv.args.every((a) => LISTING[inv.verb].includes(a))) continue;
    if (architect && (inv.verb === 'add' || inv.verb === 'commit')) {
      if (inv.assigned) refuse(`\`git ${inv.verb}\` with environment variables set in front of it: run it plainly.`);
      const territories = load(root);
      const own = refusal(territories, 'architect', agentId, null);
      if (own && own.startsWith('no territory')) refuse(own);
      (inv.verb === 'add' ? checkAdd : checkCommit)(inv, fromShell(root), territories, agentId);
      continue;
    }
    refuse(`\`git ${inv.verb || '(no verb)'}\` is not allowed here. ${allowedHere}`);
  }
}

if (require.main === module) {
  try {
    decide(JSON.parse(fs.readFileSync(0, 'utf8')));
    process.exit(0);
  } catch (e) {
    process.stderr.write(`guard-git: ${e instanceof Refused ? e.message : `could not check this command (${e.message}); blocked`}\n`);
    process.exit(2);
  }
}

module.exports = { tokenize, invocations, decide };
