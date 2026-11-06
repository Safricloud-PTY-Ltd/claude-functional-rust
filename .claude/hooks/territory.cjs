#!/usr/bin/env node
// Territories: the files each workstream may write, one file per workstream in
// .claude/territories/, written by the orchestrator. `agent <id>` lines name the architect(s)
// holding it; one optional `scratch <absolute dir>` line names the architect's scratch folder
// outside the repository; every other line that is not blank or a # comment is a glob matched
// against the repo-relative path, where * and ? cross /. Anything in no territory is the orchestrator's
// reserve. Shared by guard-writes.sh (through the CLI at the bottom) and git-policy.cjs.
'use strict';
const fs = require('node:fs');
const path = require('node:path');

// Git Bash spells C:\a as /c/a. One spelling before any comparison.
function fromShell(p) {
  const m = process.platform === 'win32' && /^\/([a-zA-Z])(\/|$)/.exec(p);
  return m ? `${m[1]}:/${p.slice(3)}` : p;
}

// The real path of p, symlinks followed, whether or not p exists yet.
function real(p) {
  let head = path.resolve(p);
  const tail = [];
  while (!fs.existsSync(head)) {
    const up = path.dirname(head);
    if (up === head) break;
    tail.unshift(path.basename(head));
    head = up;
  }
  let resolved = head;
  try { resolved = fs.realpathSync.native(head); } catch { /* keep the lexical path */ }
  return path.join(resolved, ...tail);
}

// The repo-relative, forward-slash path of p (resolved against base), or null outside root.
function repoRelative(root, p, base = root) {
  const abs = real(path.resolve(fromShell(base), fromShell(p)));
  const rel = path.relative(real(fromShell(root)), abs);
  if (rel.startsWith('..') || path.isAbsolute(rel)) return null;
  return rel.split(path.sep).join('/');
}

function globToRegExp(glob) {
  const body = glob.split('').map((c) => (c === '*' ? '.*' : c === '?' ? '.' : c.replace(/[.+^${}()|[\]\\/]/g, '\\$&'))).join('');
  return new RegExp(`^${body}$`, process.platform === 'win32' ? 'i' : '');
}

// Whether p lies strictly inside dir (both already real, absolute paths).
function inside(dir, p) {
  const rel = path.relative(dir, p);
  return rel !== '' && !rel.startsWith('..') && !path.isAbsolute(rel);
}

// A territory's `scratch <dir>` line: the one folder outside the repository its architect may
// write in (Codex briefs, notes). It must be absolute, outside the repository, and named after
// the territory, so a slip can't open a whole drive. Returns { dir } or { problem }.
function scratchOf(root, name, line) {
  if (!line) return {};
  const raw = fromShell(line.slice('scratch '.length).trim());
  if (!path.isAbsolute(raw)) return { problem: `its scratch line (${raw}) is not an absolute path` };
  const dir = real(raw);
  const same = (a, b) => (process.platform === 'win32' ? a.toLowerCase() === b.toLowerCase() : a === b);
  if (!same(path.basename(dir), name)) return { problem: `its scratch folder (${dir}) is not named after the territory, ${name}` };
  const top = real(fromShell(root));
  if (dir === top || inside(top, dir) || inside(dir, top)) return { problem: `its scratch folder (${dir}) is not outside the repository` };
  return { dir };
}

function load(root) {
  const dir = path.join(fromShell(root), '.claude', 'territories');
  let names = [];
  try { names = fs.readdirSync(dir); } catch { return []; }
  return names
    .filter((name) => fs.statSync(path.join(dir, name)).isFile())
    .map((name) => {
      const lines = fs.readFileSync(path.join(dir, name), 'utf8').split(/\r?\n/)
        .map((l) => l.trim()).filter((l) => l && !l.startsWith('#'));
      const keyword = (l) => l.startsWith('agent ') || l.startsWith('scratch ');
      return {
        name,
        agents: lines.filter((l) => l.startsWith('agent ')).map((l) => l.slice(6).trim()),
        globs: lines.filter((l) => !keyword(l)).map(globToRegExp),
        scratch: scratchOf(root, name, lines.find((l) => l.startsWith('scratch '))),
      };
    });
}

const holders = (territories, rel) => territories.filter((t) => t.globs.some((g) => g.test(rel)));

// Why this agent may not write rel, or null when it may. Only architects, test-writers and
// implementors are fenced by territories. abs, the real absolute path, matters only for a path
// outside the repository (rel null): an architect may write inside its territory's scratch folder.
function refusal(territories, agentType, agentId, rel, abs = null) {
  const shown = rel ?? abs ?? '(a path outside the repository)';
  if (agentType === 'architect') {
    const own = territories.find((t) => t.agents.includes(agentId));
    if (!own) return `no territory names architect ${agentId}. The orchestrator registers each architect after spawning it. Report BLOCKED with this id.`;
    if (rel === null) {
      if (abs && own.scratch.dir && inside(own.scratch.dir, abs)) return null;
      if (own.scratch.problem) return `${shown} is outside the repository, and territory ${own.name}'s scratch folder can't be used: ${own.scratch.problem}. Report BLOCKED to the orchestrator.`;
      if (own.scratch.dir) return `${shown} is outside your territory, ${own.name}, and outside your scratch folder, ${own.scratch.dir}.`;
      return `${shown} is outside your territory, ${own.name}, which has no scratch line. Ask the orchestrator for one in a NEEDS report.`;
    }
    const hs = holders(territories, rel);
    if (!hs.includes(own)) return `${rel} is outside your territory, ${own.name}. A shared file is a NEEDS report and another territory's file is a CROSS report, both to the orchestrator.`;
    if (hs.length > 1) return `${rel} is in territories ${hs.map((t) => t.name).join(' and ')}. Territories overlap; report BLOCKED to the orchestrator.`;
    return null;
  }
  if (agentType === 'implementor' || agentType === 'test-writer') {
    if (territories.length === 0) return null;
    if (rel === null) return `${shown} is in no architect's territory.`;
    const hs = holders(territories, rel);
    if (hs.length === 0) return `${rel} is in no architect's territory. Report it under Blocked on; your architect takes it to the orchestrator.`;
    if (hs.length > 1) return `${rel} is in territories ${hs.map((t) => t.name).join(' and ')}. Territories overlap; report it under Blocked on.`;
    return null;
  }
  return null;
}

module.exports = { fromShell, repoRelative, load, holders, refusal };

// CLI for guard-writes.sh: node territory.cjs <agent_id> <agent_type> <path>
// Prints the canonical repo-relative path (empty outside the repo), then `ok` or the refusal.
if (require.main === module) {
  const [agentId, agentType, file] = process.argv.slice(2);
  const root = process.env.CLAUDE_PROJECT_DIR || process.cwd();
  const rel = repoRelative(root, file);
  const abs = rel === null ? real(path.resolve(fromShell(root), fromShell(file))) : null;
  const why = refusal(load(root), agentType, agentId, rel, abs);
  process.stdout.write(`${rel ?? ''}\n${why ?? 'ok'}\n`);
}
