#!/usr/bin/env node
// Which git and gh commands a sub-agent may run. Called by guard-git.sh with the hook's JSON
// on stdin; exit 0 allows, exit 2 blocks with the reason on stderr. Deny by default:
//
//   main session (no agent_id)  anything
//   every sub-agent             read-only git verbs (READ_ONLY below), `git -C <dir>` and
//                               `--no-pager` only as global options; read-only gh (GH_READ
//                               below, `gh api` GET requests with no body, and help on any
//                               built-in command)
//   architect (a sub-agent)     also `git add` and `git commit`, with explicit file paths, when
//                               every file git reports the command would stage or commit lies
//                               in the architect's own territory and in no other
//
// git and gh must run as plain commands: behind xargs, env, a shell -c string, eval, or a
// command substitution the guard can't read them, so it refuses. `git add` and `git commit`
// also refuse any word bash rewrites after the guard has read it ($, globs, braces, a leading
// ~), directories, and git's own pathspec patterns, because each of those can take in files
// the check never saw. It fences mistakes by cooperative agents; it is not a sandbox.
'use strict';
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const { fromShell, repoRelative, load, refusal } = require('./territory.cjs');

const READ_ONLY = new Set(['status', 'diff', 'log', 'show', 'grep', 'blame', 'rev-parse', 'ls-files',
  'ls-tree', 'cat-file', 'describe', 'shortlog', 'merge-base', 'archive', 'rev-list', 'show-ref',
  'for-each-ref', 'name-rev', 'check-ignore']);
const LISTING = { branch: ['--show-current', '--list', '-l', '-a', '-r', '-v', '-vv', '--all'], remote: ['-v'] };
const WRAPPERS = new Set(['sh', 'bash', 'zsh', 'dash', 'eval', 'exec', 'xargs', 'env', 'command',
  'sudo', 'nice', 'nohup', 'time', 'timeout', 'pwsh', 'powershell', 'cmd', 'cmd.exe', 'find', 'parallel']);
// gh commands that only read. Anything else, aliases and extensions included, is refused.
const GH_READ = {
  pr: ['view', 'list', 'status', 'diff', 'checks'],
  issue: ['view', 'list', 'status'],
  run: ['view', 'list', 'watch'],
  workflow: ['view', 'list'],
  release: ['view', 'list'],
  repo: ['view'],
  label: ['list'],
  auth: ['status'],
  project: ['list', 'view', 'item-list', 'field-list'],
};
const GH_ANY = new Set(['help', '--help', '-h', 'version', '--version', 'status', 'search']);
// gh's own commands (gh 2.89). A help flag is honoured under these only: an alias or an
// extension is handed its arguments as they are, so `gh <alias> --help` can run anything.
const GH_BUILTIN = new Set(['agent-task', 'alias', 'api', 'attestation', 'auth', 'browse', 'cache',
  'codespace', 'completion', 'config', 'copilot', 'extension', 'gist', 'gpg-key', 'issue', 'label',
  'licenses', 'org', 'pr', 'preview', 'project', 'release', 'repo', 'ruleset', 'run', 'search',
  'secret', 'ssh-key', 'status', 'variable', 'workflow']);
// Where gh gives -h a value of its own (a hostname, a host, a homepage), so it is not help.
const GH_SHORT_H_TAKEN = new Set(['auth', 'config', 'repo']);

const isGit = (w) => /(^|[\\/])git(\.exe)?$/i.test(w);
const isGh = (w) => /(^|[\\/])gh(\.exe)?$/i.test(w);
const isTool = (w) => isGit(w) || isGh(w);
// A `$` that bash expands: a name, a positional or special parameter, or `${`.
const expandsAt = (s, i) => s[i] === '$' && /[A-Za-z0-9_{@*#?$!-]/.test(s[i + 1] ?? '');

class Refused extends Error {}
const refuse = (why) => { throw new Refused(why); };

// Shell words and operators. Quotes are honoured; redirections are dropped with their target.
// Each word records whether bash would rewrite it after the guard has read it (`expands`).
function tokenize(s) {
  const out = [];
  let cur = null;
  let quoted = false;
  let expands = false;
  const push = () => {
    if (cur !== null) out.push({ word: cur, quoted, expands });
    cur = null; quoted = false; expands = false;
  };
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
        if (s[j] === '\\' && j + 1 < s.length && '"\\$`\n'.includes(s[j + 1])) {
          if (s[j + 1] !== '\n') buf += s[j + 1];
          j += 2; continue;
        }
        if (s[j] === '`' || s.startsWith('$(', j)) refuse('a command substitution');
        if (expandsAt(s, j)) expands = true;
        buf += s[j]; j += 1;
      }
      if (j >= s.length) refuse('an unterminated quote');
      cur = (cur ?? '') + buf; quoted = true; i = j + 1; continue;
    }
    if (c === '\\' && s[i + 1] === '\n') { i += 2; continue; }   // a line continuation
    if (c === '\\' && i + 1 < s.length) { cur = (cur ?? '') + s[i + 1]; i += 2; continue; }
    if (c === '`' || s.startsWith('$(', i)) refuse('a command substitution');
    if (s.startsWith('<(', i) || s.startsWith('>(', i)) refuse('a process substitution');
    if (s.startsWith('<<', i)) refuse('a heredoc');
    if (c === '#' && cur === null) { const j = s.indexOf('\n', i); i = j < 0 ? s.length : j; continue; }
    if (c === '\n') { push(); out.push({ op: ';' }); i += 1; continue; }
    if (/\s/.test(c)) { push(); i += 1; continue; }
    // A redirection ends the word in front of it, unless that word is a descriptor number.
    if ((c === '<' || c === '>' || s.startsWith('&>', i)) && cur !== null && !/^\d+$/.test(cur)) push();
    const redirect = cur === null || /^\d+$/.test(cur) ? /^(&>>?|>>?&?|<&?)(\d+|-)?/.exec(s.slice(i)) : null;
    if (redirect) {
      cur = null; quoted = false; expands = false;
      out.push({ op: redirect[2] ? 'dup' : 'redirect' });
      i += redirect[0].length; continue;
    }
    const op = ['&&', '||', ';', '|', '&', '(', ')'].find((o) => s.startsWith(o, i));
    if (op) { push(); out.push({ op }); i += op.length; continue; }
    if (expandsAt(s, i) || '*?[{'.includes(c) || (c === '~' && cur === null)) expands = true;
    cur = (cur ?? '') + c; i += 1;
  }
  push();
  return out;
}

// Every git or gh invocation in the command:
// { tool: 'git', dir, globals, verb, args, assigned, expands } or { tool: 'gh', args }.
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
    if (seg[k].expands) refuse(`a command name bash expands (\`${head}\`)`);
    if (head === 'cd') {
      const to = seg[k + 1]?.word;
      if (to && to !== '-') dir = path.resolve(fromShell(dir), fromShell(to));
      continue;
    }
    if (!isTool(head)) {
      const rest = seg.slice(k + 1);
      const hidden = rest.find((t) => !t.quoted && isTool(t.word));
      if (hidden) refuse(`${path.basename(hidden.word)} behind \`${head}\` (if the word is only text, quote it)`);
      if (WRAPPERS.has(path.basename(head).toLowerCase()) && rest.some((t) => /\b(git|gh)\b/.test(t.word))) refuse(`git or gh behind \`${head}\``);
      continue;
    }
    if (isGh(head)) {
      found.push({ tool: 'gh', args: seg.slice(k + 1).map((t) => t.word) });
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
    found.push({
      tool: 'git', dir: at, globals, verb: seg[j]?.word ?? '', args: seg.slice(j + 1).map((t) => t.word),
      assigned, expands: seg.slice(k + 1).some((t) => t.expands),
    });
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

// The paths must be the literal files git will act on: no word bash rewrites after this check,
// no pathspec pattern or magic git expands itself, no directory that takes in whatever lands in
// it between this check and the command.
function checkLiteral(inv, paths) {
  if (inv.expands) refuse(`\`git ${inv.verb}\` with a word bash rewrites after the guard has read it ($, a glob, braces, a leading ~). Name every path literally, and put the message in single quotes.`);
  for (const p of paths) {
    if (p.startsWith(':') || /[*?[\]]/.test(p)) refuse(`\`git ${inv.verb} -- ${p}\`: git reads that as a pattern, which can take in files the guard never saw. Name each file.`);
    let stat = null;
    try { stat = fs.statSync(path.resolve(fromShell(inv.dir), fromShell(p))); } catch { /* deleted, or not there yet */ }
    if (stat?.isDirectory()) refuse(`\`git ${inv.verb} -- ${p}\` names a directory, which takes in whatever lands in it before git runs (a Codex run, a sibling). Name each file.`);
  }
}

// Absolute paths of every file the command would take in: changed in the working tree, staged,
// and (for add) untracked. git prints them relative to its own top level, which is not the
// project root when the project sits inside a larger repository.
function touched(inv, paths, untracked) {
  const top = gitLines(inv.dir, ['rev-parse', '--show-toplevel'])[0];
  if (!top) refuse('the guard could not find the repository\'s top level');
  const files = [
    ...gitLines(inv.dir, ['diff', '--name-only', 'HEAD', '--', ...paths]),
    ...gitLines(inv.dir, ['diff', '--cached', '--name-only', 'HEAD', '--', ...paths]),
    ...(untracked ? gitLines(inv.dir, ['ls-files', '--full-name', '--others', '--exclude-standard', '--', ...paths]) : []),
  ];
  return files.map((f) => path.join(fromShell(top), f));
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
  checkLiteral(inv, paths);
  checkAffected(root, territories, agentId, 'add', touched(inv, paths, true));
}

function checkCommit(inv, root, territories, agentId) {
  const takesValue = (o) => o === '-m' || o === '--message' || /^-[qsvo]+m$/.test(o);
  const { options, paths } = split(inv.args, takesValue);
  const allowed = (o) => ['-q', '--quiet', '-s', '--signoff', '-v', '--verbose', '-o', '--only', '-m', '--message'].includes(o)
    || o.startsWith('--message=') || /^-[qsvo]*m.*$/.test(o) || /^-[qsvo]+$/.test(o);
  const bad = options.filter((o) => !allowed(o));
  if (bad.length) refuse(`\`git commit\` takes \`-m <msg> -- <paths>\` and no ${bad.join(' ')}: -a, -i, --amend, --fixup, -C and the rest commit more than the files you name, or rewrite history.`);
  if (paths.length === 0) refuse('`git commit` needs at least one path; with none it commits the whole shared index, other architects\' staged files included.');
  checkLiteral(inv, paths);
  checkAffected(root, territories, agentId, 'commit', touched(inv, paths, false));
}

// `gh api` reads when it sends GET with no body. A body flag, a method other than GET, or the
// GraphQL endpoint (which takes mutations) is refused.
function checkGhApi(args) {
  for (let k = 0; k < args.length; k += 1) {
    const a = args[k];
    let method = null;
    if (a === '-X' || a === '--method') method = args[k + 1] ?? '';
    else if (a.startsWith('--method=')) method = a.slice('--method='.length);
    else if (/^-X./.test(a)) method = a.slice(2).replace(/^=/, '');
    if (method !== null && method.toUpperCase() !== 'GET') refuse(`\`gh api\` with method ${method}: sub-agents may only read from the API.`);
    if (/^(-f|-F|--field|--raw-field|--input)(=|$)/.test(a) || /^-[fF]./.test(a)) refuse('`gh api` with a request body (-f, -F, --field, --raw-field, --input) sends a POST: sub-agents may only read from the API.');
    if (a === 'graphql') refuse('`gh api graphql` can run mutations: sub-agents may only make REST GET requests.');
  }
}

// Whether the command only asks a built-in command for its help: `gh issue --help`,
// `gh issue create --help`. Once the flag is set gh prints help and runs nothing, whatever
// follows it (measured, gh 2.89: `gh label list --help --limit 1` printed help). So the help flag
// must be the first flag: one in front of it could take `--help` as its own value, as in
// `gh issue create --title --help`.
function asksForHelp(args) {
  const first = args.findIndex((a) => a.startsWith('-'));
  if (first < 1 || !GH_BUILTIN.has(args[0])) return false;
  return args[first] === '--help' || (args[first] === '-h' && !GH_SHORT_H_TAKEN.has(args[0]));
}

function checkGh(inv) {
  const [sub, verb] = inv.args;
  if (sub === undefined || GH_ANY.has(sub)) return;
  if (asksForHelp(inv.args)) return;
  if (sub === 'api') { checkGhApi(inv.args.slice(1)); return; }
  if (GH_READ[sub]?.includes(verb)) return;
  const reads = Object.entries(GH_READ).map(([s, vs]) => `${s} ${vs.join('|')}`).join(', ');
  refuse(`\`gh ${[sub, verb].filter(Boolean).join(' ')}\` is not allowed here. Sub-agents run read-only gh: ${reads}, search, \`gh api\` GET requests, and \`--help\` as the first flag of any built-in command. Anything that changes GitHub is the orchestrator's; ask for it in your report.`);
}

function decide(input) {
  const agentId = input.agent_id;
  if (!agentId) return;
  const command = input.tool_input?.command ?? '';
  if (!/\b(git|gh)\b/.test(command)) return;
  const agentType = input.agent_type ?? '';
  const root = process.env.CLAUDE_PROJECT_DIR || input.cwd || process.cwd();
  const cwd = input.cwd || root;
  const architect = agentType === 'architect';
  const allowedHere = architect
    ? 'An architect runs read-only git, plus `git add -- <files>` and `git commit -m <msg> -- <files>` on its own files. The branch, the working tree and the remote are the orchestrator\'s; ask it in a report.'
    : 'Sub-agents edit; the architect commits. Read-only git (status, diff, log, show, grep, blame) is fine. Put what you changed in your report.';
  let found;
  try {
    found = invocations(command, cwd);
  } catch (e) {
    if (e instanceof Refused) refuse(`${e.message} in a command that mentions git or gh: run them as plain commands so the guard can read them. ${allowedHere}`);
    throw e;
  }
  for (const inv of found) {
    if (inv.tool === 'gh') { checkGh(inv); continue; }
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
