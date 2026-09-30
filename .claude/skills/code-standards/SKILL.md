---
name: code-standards
description: The coding standards for this repository — functional Rust, one crate per swappable module, one concept per file, contracts before code, Result values instead of panics, pure core crates with I/O in adapter crates behind ports, the workspace and dependency rules, the rustdoc contract format, and the test conventions. Use this whenever you write, plan, review, test, or implement any Rust in this repo, including contracts, stubs, lib.rs files, tests, refactors, and one-line fixes. If you are about to create or edit a .rs or Cargo.toml file and have not read this, read it first.
---

# Code standards

Code in this repo is written by several agents working in parallel from contracts the
architect wrote first. The rules below exist so that files written by different agents,
without talking to each other, still compose into one coherent codebase, and so that every
rule that can be checked by a tool is checked by a tool (rustc, clippy, rustfmt, cargo-deny,
cargo-machete, `cargo xtask check`). Where a rule says "enforced", a hook or CI will reject the
violation; don't argue with it, report it to the architect if you think it's wrong.

## Layout

The repo is a Cargo workspace. **A module is a crate**: the thing you can swap is the thing
the compiler fences off. A crate's `pub` items are its only door, and its `Cargo.toml` is the
complete list of what it may reach.

```
Cargo.toml                 workspace: members, [workspace.lints], [workspace.dependencies]
crates/
  shared/                  kind core: primitives any crate may use — stub!. Depends on nothing.
  <vocabulary>/            kind core: the domain's shared value types and the ports (traits);
                           created by the first contribution that needs one, named for the
                           domain (`domain-types`, `order-types`)
  <module>/                one swappable thing: a parser, a pricing rule set, a codec, ...
    Cargo.toml             declares its kind; its [dependencies] are its import rules
    src/
      lib.rs               the crate's door: `mod` lines and `pub use` only. Architect-owned.
      types.rs             the crate's types, error kinds, and their constructors
      <fn_name>.rs         one primary `pub fn`, named after the file
      <fn_name>_tests.rs   its contract tests, colocated; owned by the test-writer
      <concept>/mod.rs     a sub-module when a concept needs several files; same rules inside
    tests/
      api.rs               black-box test through the public API: what a replacement must pass
  store-postgres/          kind adapter: one external thing behind one port, plus its fake
  app/                     kind app: the composition root; wires adapters into core
  test-support/            kind tool: test helpers more than one test file needs; dev-dependency only
xtask/                     kind tool: `cargo xtask check`
```

- Every crate declares `[package.metadata.standards] kind`, one of `core`, `adapter`,
  `app`, `tool` (enforced).
- A file is named after its primary export: `line_total.rs` exports `line_total`.
  `snake_case.rs` always. A type that needs its own file is named after the type, snake-cased.
- One primary `pub fn` per file. A private helper may live in the same file when the
  architect puts it there; it is still a contract, with a doc comment and a `@stub` marker.
  Until its caller has a body it is dead code, so its stub carries a temporary
  `#[allow(dead_code, reason = "@stub <n>: delete this line with the stub")]`, which the
  implementor removes along with the marker.
- Sub-modules use `<concept>/mod.rs`, never `<concept>.rs` beside a `<concept>/` directory
  (enforced), so a concept is always one directory entry.
- New files, new directories, new crates and every `Cargo.toml` are architect decisions.
  Implementors and test-writers don't create them (enforced); they report what they need.
- Large literal tables (lookup tables, code books, rate schedules) live in their own
  `<name>_table.rs` file holding one `pub(crate) const` and nothing else.

## Modules

A module is a boundary with something behind it. It earns its crate by hiding one of three
things; a crate that hides none of them is a file in another crate.

- **A representation.** The crate exports an opaque type (a struct with private fields, or a
  newtype) and the functions that make, read, and combine it; callers never see the shape.
  `Timestamp` is a newtype over `i64` ticks today and could be a rational tomorrow without a
  caller changing. Structs with `pub` fields are for boundary data and for types whose shape
  *is* the contract, never for a module's own data.
- **A dependency.** Third-party crates and OS access appear only in `adapter` crates (and
  `app`, `tool`). A `core` crate depends at runtime on workspace `core` crates and nothing else
  (enforced); dev-dependencies are free. Swapping the database driver, the HTTP client, or the
  file reader is one crate's diff.
- **A choice of implementation.** A port with a real adapter and an in-memory fake, or an
  algorithm with a plausible alternative (two pricing engines, two search strategies). The fake is mandatory for
  every port: writing it is what proves the port doesn't leak the real implementation, and
  tests use it instead of mocks.

The architect's test for a proposed crate: *if this were swapped for another implementation,
what would change on the other side?* If nothing, there is no boundary to draw.

- `lib.rs` is the contract. It is written first, like any contract, and contains only a crate
  doc comment, inner attributes, `mod` lines and `pub use` lines. What `lib.rs` doesn't
  re-export doesn't exist outside. Sub-modules are private (`mod x;`), their items re-exported
  by name; no glob re-exports (enforced).
- Ports are traits declared by the consumer side, in the vocabulary crate (or the consuming
  crate's `types.rs`), never by the adapter. The adapter crate exports the real implementation
  and `Fake<Name>` implementing the same trait. A port that is a single operation is a trait
  with one method, so a closure-free fake stays trivial.
- Sibling implementations never depend on each other: one parser crate never reaches into
  another. They meet only through the vocabulary crate. The `app` crate is the one place that
  names concrete implementations, with one exception: a dispatch crate that picks one of
  several implementations at runtime (by a file's first bytes, a declared format, a
  configured provider) names every one of them, because choosing between them is its whole
  job.
- Every crate has `tests/api.rs`, which by construction can only see the public API. It is the
  acceptance suite a replacement would have to pass.
- Inside a crate, files reach each other with `crate::` or `super::` paths. Items are
  `pub(crate)` unless `lib.rs` re-exports them, in which case they are `pub`.

## Dependencies

- Third-party crates are declared once in the root `[workspace.dependencies]` and inherited
  with `{ workspace = true }`. Adding one is an architect decision recorded in the plan, and
  `cargo deny check` must pass (licences, advisories, duplicate versions).
- `core` crates: `#![forbid(unsafe_code)]` in `lib.rs`, no runtime third-party dependencies,
  no `std::fs`, `std::net`, `std::process`, `std::env`, `std::time::SystemTime`/`Instant`,
  threads or channels. Bytes in, values out.
- `adapter` crates: exactly one external concern each. `unsafe` only with an item-level
  `#[allow(unsafe_code)]` and a `// SAFETY:` comment, and only when the foreign API demands it.
- No dependency cycles (cargo refuses them). Nothing depends on an `app` or `tool` crate,
  except that any crate may take the `tool` crate `test-support` as a dev-dependency.

## Functions

- Free functions are the default: `pub fn name(params) -> ReturnType`. `impl` blocks are for
  trait implementations (ports, `From`, `Display`) and for an opaque type's constructors and
  accessors when the architect's contract puts them there. No inherent-method grab-bags.
- Up to three parameters (enforced). Beyond that, one options struct with named fields.
- Inputs are borrowed immutably or taken by value: `&[T]`, `&T`, `T`. A `&mut` parameter
  appears only where the architect's contract says so with a measured reason in `# Remarks`
  (a hot loop writing into a large caller-owned buffer is the expected case).
- A `let mut` that never escapes the function is fine: mutation nobody can observe is still a
  pure function, and the borrow checker proves nobody can. Prefer iterator chains
  (`map`/`filter`/`fold`/`zip`/`chunks_exact`) where they read as well; use a loop where the
  loop is clearer or measurably faster.
- No `static mut`, no interior mutability (`Cell`, `RefCell`, `Mutex`, `RwLock`), no globals,
  no `lazy`/`once` state in `core` crates (enforced for the named types).
- No boolean parameters (enforced). A flag means two functions or an enum.
- One level of abstraction per function. A function either orchestrates calls to other
  functions or does one piece of work itself, not both.
- Target under 40 lines per function (enforced) and under 150 per file, tables and
  `types.rs` aside (declarations are not logic). Hitting
  a limit is a signal to extract, not to `#[allow]`.
- `#[must_use]` on every pure function that returns a value (enforced by clippy pedantic).
- Naming: functions are verbs (`decode_block`, `parse_header`), predicates start with
  `is_`/`has_`, constructors start with `make_`/`to_` or are `Type::new`/`Type::try_from`,
  types are nouns.
- Every named function is a contract the architect wrote. An implementor writes the body of
  the contract it holds and declares no functions of its own: no `fn`, no named closure bound
  with `let f = |..|`, no private helper, however small. Closures passed straight to `map`,
  `filter`, `fold`, `and_then`, `map_err` and the like are expressions, not functions, and are
  fine. A function you want to name is a NEEDS report to the architect, who reuses one that
  exists, generalises one that almost does, or writes a new stub.

## Types

- Enums with data over trait-object hierarchies. `match` exhaustively; no `_ =>` arm over an
  enum this workspace owns, so adding a variant is a compile error everywhere it matters.
- Values that must not be mixed are newtypes: `pub struct OrderId(u64);` with a constructor in
  the crate's `types.rs`. Units are types too: `Money`, `Quantity`, `Timestamp`.
- Derive what a value type should have: `Debug, Clone, PartialEq, Eq` as a floor; `Copy` when
  it is small and plain; `Hash`/`Ord` when meaningful. Tests compare values with `assert_eq!`.
- No `as` for narrowing numeric casts (enforced by clippy pedantic): `u8::try_from(x)`,
  `u32::from(x)`. Where a cast is provably in range and hot, the contract's `# Remarks` says so
  and the line carries an `#[allow]` with the reason.
- Absence is `Option<T>`. No sentinel values (`-1`, `0`, empty string) standing for "none".
- Arithmetic that can overflow on hostile input uses `checked_*`/`saturating_*`/`wrapping_*`
  explicitly. Every byte that arrives from outside the process is hostile input.

## Errors

Production code never panics. A function that can fail returns `Result<T, E>`; callers must
handle it (enforced: `unused_must_use`).

- Error values are plain data: a struct per kind, carrying what a caller needs to report it
  (`pub struct UnexpectedEof { pub needed: usize, pub available: usize }`). Each crate declares
  its kinds in `types.rs`, derives `Debug, Clone, PartialEq, Eq`, and implements `Display` and
  `std::error::Error` so an `app` can print them. No `thiserror`, no `anyhow` in `core`.
- A function's error type names exactly the kinds it can produce. One kind: return that
  struct. Several: a small enum named for the function (`ParseHeaderError`) whose variants wrap
  the kind structs, with `From` impls so `?` composes. Never the whole crate's union.
- Denied everywhere outside tests (enforced): `unwrap`, `expect`, `panic!`, `todo!`,
  `unimplemented!`, `unreachable!`, slice indexing that can panic (`v[i]`, `&s[a..b]`; use
  `get`, iterators, `split_at_checked`, or fixed-size arrays with constant indices),
  `process::exit`, `dbg!`, `println!`/`eprintln!` (only an `app` or `tool` `main.rs` prints,
  with an item-level `#[allow]`).
- The adapter edge is the only place foreign errors exist. Convert them to a kind struct at the
  boundary with `map_err`; never leak a third-party error type through a port.
- The single sanctioned panic is `shared::stub!`, and `cargo xtask check` fails while one
  remains.

## Contracts

The architect writes a contract for every function before any implementation exists. A
contract is the signature, its doc comment, a stub body, and the line that attaches its test
file. Contracts are the interface between agents, so their format is fixed:

```rust
use shared::stub;

use crate::types::{InvalidDiscount, LineItem, Minor};

/// Computes the total of the line items after a fractional discount.
///
/// # Parameters
/// - `items`: line items to total. May be empty, in which case the total is 0.
/// - `discount`: fraction to remove, in [0, 1]. 0 means no discount.
///
/// # Returns
/// The discounted total in minor units, rounded half-up.
///
/// # Errors
/// - [`InvalidDiscount`] when `discount` is outside [0, 1] or is NaN.
///
/// # Remarks
/// Pure. O(n) in `items`.
#[must_use]
pub fn line_total(items: &[LineItem], discount: f64) -> Result<Minor, InvalidDiscount> {
    // @stub 2.3
    stub!("line_total", items, discount)
}

#[cfg(test)]
#[allow(clippy::too_many_lines, reason = "a test may spell out its whole expected value")]
#[path = "line_total_tests.rs"]
mod tests;
```

- The summary line says what the function computes, in one sentence.
- `# Parameters` gives every parameter its constraints and edge cases (empty, zero, bounds).
- `# Returns` describes the ok value. `# Errors` lists one line per error kind with the
  condition that produces it. Types are never repeated in prose; the signature has them.
- `# Remarks` carries anything the implementor must know that the types don't say: purity,
  complexity, rounding, ordering guarantees, bit-exactness against a named reference.
- `// @stub <phase>.<item>` is the first line of the body and names the brief that will
  implement it. Under the orchestrator the marker carries the workstream too,
  `// @stub billing-2.3`, so parallel architects' items can't be confused. `stub!` panics with `not implemented: <name>`; that is the only way a stub may
  fail. The contract's parameters are passed through so the unused-variable lint stays on for
  real code. The implementor deletes the marker line, the `stub!` call and the `use shared::stub;`
  line when it implements the body. The marker is how hooks recognise a file that is open for
  implementation (enforced).
- The architect creates the `_tests.rs` file alongside the contract (a one-line `//!` header is
  enough) because the `#[path]` line doesn't compile without it. The test-writer fills it.
- Implementors change the body and nothing else: not the signature, the doc comment, the
  `mod tests` lines, or the tests, and they add no functions. The one exception is imports: an
  implementor may add the names its body uses to the file's top-level `use` lines (never a
  `use` inside the body), and lists them under Deviations. If any of
  those needs to change, that's a report to the architect, not an edit.

## Adapters and dependencies

- A port is a trait; an effect is a function generic over the ports it needs, taking them
  first: `pub fn play<S: ByteSource, P: Presenter>(source: &mut S, presenter: &mut P, ...)`.
  The `app` crate builds the real adapters and passes them in. This is the whole
  dependency-injection story; there is no container and no global.
- Clock, randomness, environment, filesystem and devices are ports, never direct calls in
  `core`: a `Clock` trait with `fn now(&self) -> Timestamp`, faked in tests by a fixed value.
- Bytes entering the system are parsed into typed values by a `core` parser that returns a
  `Result`. After that point the types are trusted; downstream code does not re-validate.
- Generics (`impl Trait`, `<T: Port>`) are the default for ports; `dyn Trait` when the choice
  is made at runtime (picking a parser from a probed format). The contract decides.
- The `app` crate reads arguments and environment once, into a typed config. Nothing else
  reads either.

## Tests

- One `<fn_name>_tests.rs` per contract, colocated, attached by the contract's `#[path]` line.
  It imports what it tests by name, `use super::line_total;`, never `use super::*;`.
- One `tests/api.rs` per crate, exercising the public API as a caller would. Ports are
  exercised through the crate's fake, never a mock.
- Tests are written from the contract, not the implementation. The test-writer reads the
  signature and doc comment and nothing else.
- Each `# Parameters` edge case and each `# Errors` kind gets at least one `#[test]`. Assert on
  the whole value: `assert_eq!(line_total(&[], 0.0), Ok(Minor::new(0)))`,
  `assert_eq!(r, Err(InvalidDiscount { discount: 1.5 }))`.
- When the contract states an invariant (idempotent, order-independent, bounded, inverse of
  another function, never panics on arbitrary bytes), add a `proptest!` property. Every parser
  gets "arbitrary bytes never panic" for free. When a property finds a real bug, pin the
  shrunk case as a named `#[test]`; `proptest-regressions/` is ignored, not committed.
- Where a reference implementation exists (a spec's reference code, a mature tool), outputs
  are tested against it exactly: small fixtures under the crate's `tests/fixtures/`, with the
  expected output (or its hash) produced by the reference tool and recorded in the test. The
  reference tool is never a dependency. Measure an expected value before a contract or test
  asserts it; never write down what a reference "should" produce.
- No mocking crates. Pure functions don't need them; effects take the fake.
- A test helper is never copied. One that two test files need (a builder, a proptest strategy,
  an encoder that is a parser's inverse) lives in the `test-support` crate and is imported
  from there; if it isn't there, report it to the architect, who writes its contract.
  A helper over a crate's own private types, which `test-support` can't name, lives in that
  crate's `#[cfg(test)] mod testing` instead. The same goes for the vocabulary crate's
  colocated tests: `test-support` depends on it, so they would see a second copy of its
  types. Shared fakes of the ports (a scripted source that replays given values), constant
  tables, and builders for fixture bytes all live in `test-support`.
- A test's name states the behaviour: `returns_invalid_discount_when_discount_is_above_one`.
  Names like `works` or `handles_edge_case` are rejected in review.
- `unwrap`/`expect`/indexing/`panic!` are allowed in tests; that is how a test fails. The
  40-line limit is lifted for tests by the `#[allow]` on the contract's `mod tests` line (and
  an inner `#![allow]` the architect puts at the top of `tests/api.rs`): the limit exists to
  force extraction in production code, not to squeeze a test that spells out its expectation.

## Doc comments and comments

- Every `pub` and `pub(crate)` function has a doc comment in the contract format above; every
  other public item has at least a summary line (enforced: `missing_docs`). Every file that is
  a module root starts with a `//!` line saying what the module is.
- Comments explain why, never what. If a comment describes what the next line does, the line
  needs a better name instead. Cite the spec section for anything that implements one:
  `// RFC 9110 §15.5.1`. Code ported or adapted from a reference implementation says
  where it came from, `// Ported from zlib inflate.c, inflate_fast`, so whoever debugs it
  can compare against the original. Documents and reference code live under `docs/`; a brief
  names the ones it relies on by path.
- No commented-out code, no TODOs in committed code. Open work is a GitHub issue.

## Review checklist

Before reporting a file done, check it against this list; the architect will.

- [ ] File named after its primary export; one primary export; under 150 lines (tables aside).
- [ ] Every export has the contract-format doc comment; no `@stub` or `stub!` left behind.
- [ ] No `unwrap`, `expect`, `panic!`, `todo!`, panicking index, narrowing `as`, `unsafe`,
      interior mutability, boolean parameter, or wildcard arm over a workspace enum.
- [ ] No function declared by an implementor; every named function has a contract.
- [ ] Failure paths return `Result` with a narrow, named error type that carries its data.
- [ ] A `core` crate's `Cargo.toml` has only workspace `core` crates under `[dependencies]`;
      crate-owned data is opaque; nothing outside `lib.rs`'s re-exports is `pub`.
- [ ] Tests cover every `# Parameters` edge case and every `# Errors` kind; parsers have a
      never-panics property; no mocks.
- [ ] `cargo clippy -p <crate> --all-targets -- -D warnings`, `cargo fmt --check`, and the
      file's tests pass; nothing outside the brief was touched.
