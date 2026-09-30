/// The body of a contract nobody has implemented yet.
///
/// Expands to a panic with the message `not implemented: <name>`, after moving every parameter
/// into an unused binding, so neither the unused-variable lint nor clippy's pass-by-value lint fires
/// on a contract that has no body yet, and both stay on for real code. A contract's stub body
/// is `// @stub <phase>.<item>` followed by `stub!("fn_name", param_a, param_b)`. The name is a
/// string literal, joined to the message at compile time, so a stub also works as the body of a
/// `const fn` contract.
///
/// This is the only sanctioned panic in the workspace. `cargo xtask check` fails while any
/// `stub!` call remains, so a stub cannot reach `main`.
#[macro_export]
macro_rules! stub {
    ($name:literal $(, $param:expr)* $(,)?) => {{
        $( let _unused = $param; )*
        ::core::panic!(::core::concat!("not implemented: ", $name))
    }};
}
