// Shared by the hook tests (*.test.cjs): a throwaway project with the hooks copied into it, stub
// tools first on PATH, and one way to run a hook on hand-built input. Run every suite with
//   node --test .claude/hooks/
// from any shell. Each test builds its own project under the temp directory and removes it.
'use strict';
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const HOOKS = __dirname;
const isTestFile = (name) => name.endsWith('.test.cjs') || name === 'hook-test-kit.cjs';

// The bash the hooks run under. On Windows a bare `bash` can be WSL's (System32 comes before
// Git on a PowerShell PATH), and WSL can't run these scripts against a Windows node. So ask git
// where it is installed and take the bash beside it. HOOK_TEST_BASH overrides.
function findBash() {
  if (process.env.HOOK_TEST_BASH) return process.env.HOOK_TEST_BASH;
  if (process.platform !== 'win32') return 'bash';
  const r = spawnSync('git', ['--exec-path'], { encoding: 'utf8' });
  let dir = r.status === 0 ? r.stdout.trim() : '';
  // <git>/mingw64/libexec/git-core, and bash is <git>/bin/bash.exe.
  for (let up = 0; up < 4 && dir; up += 1) {
    const candidate = path.join(dir, 'bin', 'bash.exe');
    if (fs.existsSync(candidate)) return candidate;
    dir = path.dirname(dir);
  }
  return 'bash';
}
const BASH = findBash();

// A throwaway project with this directory's hooks, and none of their tests, in .claude/hooks.
// The hooks run from CLAUDE_PROJECT_DIR, so a test never touches the repository it runs in.
function makeProject(t) {
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), 'hook-test-'));
  t.after(() => fs.rmSync(repo, { recursive: true, force: true }));
  const hooks = path.join(repo, '.claude', 'hooks');
  fs.mkdirSync(hooks, { recursive: true });
  fs.readdirSync(HOOKS).filter((name) => !isTestFile(name))
    .forEach((name) => fs.copyFileSync(path.join(HOOKS, name), path.join(hooks, name)));
  return repo;
}

// Writes <repo>/<rel>, a forward-slash path, making its directories.
function write(repo, rel, body = '') {
  const file = path.join(repo, ...rel.split('/'));
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, body);
  return file;
}

// The same directory in its other Windows spelling, or null when it has only one. The temp
// directory is often spelled with a short (8.3) name, C:\Users\SOMEON~1\..., and the real path
// is the long one. A hook that compares paths has to cope with both arriving at once.
function otherSpelling(p) {
  const real = fs.realpathSync.native(p);
  return real === p ? null : real;
}

// The name PATH has in this process's environment: `Path` under PowerShell, `PATH` under bash.
const pathKey = () => Object.keys(process.env).find((k) => k.toUpperCase() === 'PATH') ?? 'PATH';

// Puts stub tools first on PATH, so a hook under test never reaches the machine's own cargo or
// rustfmt. Each stub is a shell script; it should append its arguments to $STUB_LOG. Returns the
// environment to run the hook with, and a reader for the calls the stubs recorded, one per line.
function stubTools(repo, tools) {
  const dir = path.join(repo, '.stub-bin');
  fs.mkdirSync(dir, { recursive: true });
  Object.entries(tools).forEach(([name, body]) => fs.writeFileSync(path.join(dir, name), body, { mode: 0o755 }));
  const log = path.join(repo, 'stub-calls.log');
  const key = pathKey();
  return {
    env: { [key]: `${dir}${path.delimiter}${process.env[key] ?? ''}`, STUB_LOG: log.split(path.sep).join('/') },
    calls: () => (fs.existsSync(log) ? fs.readFileSync(log, 'utf8').split('\n').filter(Boolean) : []),
    reset: () => fs.rmSync(log, { force: true }),
  };
}

// Runs one hook script from the project's own copy, with the hook's JSON on stdin.
// `project` is the spelling CLAUDE_PROJECT_DIR is given in; it defaults to the repo's own.
function runHook(repo, script, input, { env = {}, project = repo } = {}) {
  return spawnSync(BASH, [path.join(repo, '.claude', 'hooks', script)], {
    input: typeof input === 'string' ? input : JSON.stringify(input),
    env: { ...process.env, ...env, CLAUDE_PROJECT_DIR: project },
    encoding: 'utf8',
  });
}

module.exports = { BASH, makeProject, write, otherSpelling, stubTools, runHook };
