# AGENTS.md: Codex in this repository

Codex reads this file at the start of every run. This repo's agents are defined for Claude
Code, and Codex does two of their jobs:

- **A brief.** An architect hands you one test-writer brief (`T-…`) or one implementor brief
  (`I-…`) through `.claude/codex/run-brief.sh`. Your prompt names the brief id and the role.
- **A pull request review.** Your prompt says you are the `codex` reviewer.

Everything under **Everyone** applies to both. **Briefs** applies only to a brief.

## Everyone

Read these before you do anything else:

1. `CLAUDE.md`, the workspace instructions. Its **Rule zero**, **Habits** and **Code**
   sections bind you as they bind every agent here. Its **Commands** are for reference:
   of those, you run only the narrow ones your brief or the review procedure names (see
   below; never `cargo xtask check` or a release build). **Who does what** and the
   settings describe the loop around you; you don't run that loop.
2. `.claude/skills/code-standards/SKILL.md`, in full. Claude agents get it preloaded, but you
   have to read it yourself. It is the rulebook your work is checked against.
3. Your role:
   - brief `T-…`: `.claude/agents/test-writer.md`
   - brief `I-…`: `.claude/agents/implementor.md`
   - the reviewer: **Doing a review** in `CONTRIBUTING.md`, followed exactly

   Skip the YAML front matter at the top of the role files. `tools`, `maxTurns` and
   `effort` are Claude Code settings.

**The checkout is shared.** Other agents, Claude and Codex, are editing other files in this
checkout on the same branch while you run. Files you didn't write may change under you, and
some crates may not compile for a minute while a sibling is mid-write. None of that is
yours to fix.

- Change no file the brief or the review procedure doesn't give you. That includes
  formatting other files, "fixing" a sibling's compile error, and deleting stray files.
- Keep LF line endings (`.gitattributes` forces them). Edit with your patch tool, not with
  shell redirection or a script that rewrites files.
- Git is read-only for you: `status`, `diff`, `log`, `show`, `grep`, `blame`, `ls-files`,
  `rev-parse`. Never `add`, `commit`, `stash`, `restore`, `checkout`, `switch`, `reset`,
  `clean`, `rebase` or `push`. In a shared checkout, a reverted file is somebody else's
  lost work.
- Never run `cargo fmt --all`, which rewrites everyone's files. Never run `cargo clean`,
  which pulls `target/` out from under every running build. Never run `cargo xtask check`,
  a workspace-wide test run or a release build. Use the narrow commands your brief or the
  review procedure names.
- Never kill a process you didn't start. Other agents' `cargo` builds are running.

**Import, don't duplicate.** Import what already exists, such as builders and strategies
from `crates/test-support` and the functions your brief lists. When the thing you'd import
doesn't exist, you don't write it. A test-writer reports it as CLARIFY, an implementor as
NEEDS, and the architect decides where it lives.

## Briefs

**The guards don't run for you.** In Claude Code, hooks in `.claude/hooks/` block a
sub-agent's write outside its lane and block state-changing git. After every edit they also
format-check the file, lint its crate and run its tests. None of those hooks sees Codex.
The rules still hold, so you keep them yourself. After your run, the architect diffs the
tree and undoes anything outside your brief, so a stray edit only costs a round trip.

- **Write only the paths on your brief's `Write:` line.** Create no other file: no scratch
  file, no notes, no new module, no helper. If you need another file changed, put it in
  your report.
- **Test-writer:** you may write only a `<name>_tests.rs` beside an existing `<name>.rs`
  contract, or a `crates/<crate>/tests/*.rs` in a crate that has a `src/lib.rs`. You never
  edit a contract, a type, `lib.rs`, `mod.rs` or a `Cargo.toml`. A dev-dependency you need
  is a CLARIFY.
- **Implementor:** you may write only the existing files on your `Write:` line, the ones
  that carry (or carried, before your edit removed it) a `// @stub` tagged with your item
  number. You may keep editing such a file to fix its clippy findings once the marker is
  gone. You never touch a test, create a file, or edit a `Cargo.toml`. You
  declare no function, not even a private one: a function you want named is a NEEDS.
- **Validate after every edit,** with your brief's own commands: `rustfmt --check
  --edition 2024 <your file>`, the crate's clippy, and your tests. That replaces the
  post-edit hook. rustfmt follows a contract's `#[path]` test module, so a plain `rustfmt
  <your file>` can reformat a tests file you don't own, which lands in `.stray` and fails
  the run. Run `rustfmt --check --edition 2024 <your file>`, and fix the formatting it
  reports in your own file by hand. If it flags only the tests file, leave that file alone
  and say so in your report.
- **A compile error in a file you don't own** is a sibling mid-write. Wait a minute, retry
  once, then report it under `Not done`.
- **The network is off** in your sandbox. Cargo can't fetch a crate. A missing dependency is
  a NEEDS or a CLARIFY, never a workaround.

**Your report is your last message.** Use exactly the report format at the end of your role
file, with nothing after it, starting with the `Status:` line. The architect reads only
that last message, so a report in an earlier message is lost. Start every other text block
with your brief id (`T-app-5.3:`), as the role file's **Voice** says.

**Being resumed.** When the architect answers your NEEDS, CLARIFY or BLOCKED, the same
session continues with the answer. Carry on with the same brief and its `Write:` line,
which the answer may widen. When you're done, end with a fresh report.
