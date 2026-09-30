# shared

Primitives any crate in the workspace may use. Depends on nothing.

Today it holds one thing: the `stub!` macro, the body of every contract that has no
implementation yet. It panics with `not implemented: <name>` and is the only sanctioned panic in
the workspace; `cargo xtask check` fails while any `stub!` call remains.

## Consumes

Nothing. No dependencies, workspace or third-party.

## Consumed by

Any crate whose contracts are still stubs, as a normal dependency, removed once the last body
lands. No crate on `main` depends on it, by construction.

## Boundary

- `stub!("fn_name", param_a, param_b)` — a `#[macro_export]` macro; satisfies any return type
  and keeps the unused-variable and pass-by-value lints quiet.

No types, no traits.
