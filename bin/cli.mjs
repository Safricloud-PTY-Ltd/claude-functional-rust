#!/usr/bin/env node
// Plain JavaScript on purpose: npm installs this package under node_modules, where Node
// refuses to strip types (ERR_UNSUPPORTED_NODE_MODULES_TYPE_STRIPPING), so a .ts file
// cannot be a bin. Staying JS keeps this a single file with no build step and no runtime
// dependencies, which is what makes `npx` here take seconds rather than a minute. Node is
// already a requirement: the Claude Code hooks this installs run on it.
import { cpSync, existsSync, mkdirSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

/** What the scaffold ships into a new project, relative to the package root. */
const PAYLOAD = ['CLAUDE.md', 'CONTRIBUTING.md', 'AGENTS.md', '.claude'];

/** Payload paths that belong to the source repo alone, or to a running session, and must never be shipped. */
const EXCLUDED = [
  '.claude/agent-memory',
  '.claude/locks',
  '.claude/territories',
  '.claude/settings.local.json',
];

const HELP = `Bootstrap a project with the Claude Code functional-Rust scaffold.

  npx github:Safricloud-PTY-Ltd/claude-functional-rust [directory] [--force]

  directory   Where to write CLAUDE.md, CONTRIBUTING.md, AGENTS.md and .claude/. Defaults to the
              current directory.
  --force     Overwrite files that already exist. Without it they are left alone.
  --help      Show this message.`;

/**
 * Reads the command line into options.
 *
 * @param argv - Arguments after the program name. The first entry that is not a flag is the
 * target directory; when there is none the target is the current directory.
 * @returns The target resolved to an absolute path, and the force and help flags.
 * @remarks Reads the process working directory to resolve a relative target.
 */
const parseArgs = (argv) => ({
  target: resolve(argv.find((arg) => !arg.startsWith('-')) ?? '.'),
  force: argv.includes('--force'),
  help: argv.includes('--help') || argv.includes('-h'),
});

/**
 * Lists every payload file under one path, depth first, skipping excluded paths.
 *
 * @param root - Absolute path of the package the payload is read from.
 * @param rel - Path relative to `root`. A file yields itself, a directory yields everything
 * beneath it, and a missing or excluded path yields nothing.
 * @returns Slash-separated paths relative to `root`.
 * @remarks Recursive. Reads directory entries but writes nothing.
 */
const collectFiles = (root, rel) => {
  const posix = rel.split(sep).join('/');
  const abs = join(root, rel);
  const shipped = !EXCLUDED.some((ex) => posix === ex || posix.startsWith(`${ex}/`));
  if (!shipped || !existsSync(abs)) return [];
  return statSync(abs).isDirectory()
    ? readdirSync(abs).flatMap((entry) => collectFiles(root, join(rel, entry)))
    : [posix];
};

/**
 * Decides, for each payload file, whether the target already has it.
 *
 * @param files - Payload paths relative to the package root.
 * @param roots - Absolute directories to copy from and to.
 * @returns One action per input file, in the same order, marked `skip` where the target
 * already has that file and `create` where it does not.
 * @remarks Reads the file system to test existence but writes nothing. Overwriting is not
 * decided here; `forceAll` re-marks the plan when the caller asked for it.
 */
const planCopy = (files, roots) =>
  files.map((rel) => ({
    kind: existsSync(join(roots.to, rel)) ? 'skip' : 'create',
    from: join(roots.from, rel),
    to: join(roots.to, rel),
  }));

/**
 * Re-marks every action as `create`, so existing files are overwritten.
 *
 * @param plan - The plan to re-mark.
 * @returns A new plan in which nothing is skipped.
 * @remarks Pure. This is what `--force` means.
 */
const forceAll = (plan) => plan.map((action) => ({ ...action, kind: 'create' }));

/**
 * Writes every action marked `create`, making parent directories as needed.
 *
 * @param plan - The actions to apply. Entries marked `skip` are left alone.
 * @returns Nothing; the effect is on disk.
 * @remarks The only function here that writes. Not atomic: a failure part way through
 * leaves the files already copied in place, which re-running with `--force` repairs.
 */
const applyPlan = (plan) => {
  plan
    .filter((action) => action.kind === 'create')
    .forEach((action) => {
      mkdirSync(dirname(action.to), { recursive: true });
      cpSync(action.from, action.to);
    });
};

/**
 * Renders what happened and what to run next.
 *
 * @param plan - The actions that were applied.
 * @param target - Absolute path the payload was written to.
 * @returns A report for stdout.
 * @remarks Pure.
 */
const formatReport = (plan, target) => {
  const created = plan.filter((action) => action.kind === 'create').length;
  const skipped = plan.length - created;
  const kept = skipped === 0 ? '' : `\n  kept ${skipped} existing file(s); --force overwrites`;
  return [
    `\n  wrote ${created} file(s) to ${target}${kept}`,
    '',
    'Next:',
    '  git init            (if this is not a repository yet)',
    '  claude              starts the orchestrator; ask it to install the scaffold',
    '',
  ].join('\n');
};

/**
 * Copies the scaffold into the target directory and reports what it did.
 *
 * @param argv - Arguments after the program name.
 * @param packageRoot - Absolute path of this package, which is the payload's source.
 * @returns Nothing; prints to stdout and sets a non-zero exit code when it refuses.
 * @remarks Refuses to copy the package over itself, which would otherwise be a no-op that
 * reads as success.
 */
const run = (argv, packageRoot) => {
  const { target, force, help } = parseArgs(argv);
  if (help) {
    console.log(HELP);
    return;
  }
  if (target === packageRoot) {
    process.exitCode = 1;
    console.error('Refusing to copy the scaffold over itself.');
    return;
  }
  const found = PAYLOAD.flatMap((entry) => collectFiles(packageRoot, entry));
  const base = planCopy(found, { from: packageRoot, to: target });
  const plan = force ? forceAll(base) : base;
  applyPlan(plan);
  console.log(formatReport(plan, target));
};

// The shell edge: the one place allowed to turn a thrown error into a message.
try {
  run(process.argv.slice(2), resolve(dirname(fileURLToPath(import.meta.url)), '..'));
} catch (error) {
  process.exitCode = 1;
  console.error(`Could not write the scaffold: ${error?.message ?? error}`);
}
