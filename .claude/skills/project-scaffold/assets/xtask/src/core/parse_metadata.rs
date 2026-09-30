use serde_json::Value;

use crate::types::{CrateKind, Dependency, DependencyScope, InvalidMetadata, Package};

/// Reads the workspace's packages out of `cargo metadata --format-version 1 --no-deps` JSON.
///
/// # Parameters
/// - `json`: the command's standard output. The fields read are, for each element of the
///   top-level `packages` array: `name` (string); `metadata` (may be `null` or absent), within
///   it `standards.kind` (string); and `dependencies` (array), within each element `name`
///   (string) and `kind` (`null` or absent for a runtime dependency, `"dev"`, or `"build"`).
///   Every other field is ignored, whatever its type.
///
/// # Returns
/// One [`Package`] per element of `packages`, in the same order, each with its dependencies in
/// the order given. A package's `kind` is `Some` for exactly `"core"`, `"adapter"`, `"app"`
/// and `"tool"`; it is `None` when `metadata` is `null` or absent, when it has no
/// `standards` table, when that table has no `kind`, or when `kind` is any other string. It
/// is also `None` when any of `metadata`, `standards` or `kind` has an unexpected type (a
/// number where a table or string belongs): a kind that cannot be read is a missing kind,
/// which the purity rule reports, not a parse failure. An empty `packages` array gives an
/// empty vector.
///
/// # Errors
/// - [`InvalidMetadata`] when `json` is not valid JSON, when there is no top-level `packages`
///   array, when a package has no string `name` or no `dependencies` array, when a dependency
///   has no string `name`, or when a dependency's `kind` is anything other than `null`,
///   absent, `"dev"` or `"build"` (another string, a number, a table). `reason` is never
///   empty.
///
/// # Remarks
/// Pure. Never panics, whatever the input.
pub fn parse_metadata(json: &str) -> Result<Vec<Package>, InvalidMetadata> {
    let root = serde_json::from_str::<Value>(json).map_err(|error| InvalidMetadata {
        reason: error.to_string(),
    })?;
    root.get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| InvalidMetadata {
            reason: "the top-level `packages` key is missing or is not an array".to_owned(),
        })?
        .iter()
        .map(parse_package)
        .collect()
}

/// Reads one element of the `packages` array.
///
/// # Parameters
/// - `value`: the element. `name` and `dependencies` are required; the kind is read leniently
///   through `metadata`, then `standards`, then `kind`, any missing or wrongly-typed link
///   giving `None`, and a string is mapped with [`to_crate_kind`].
///
/// # Errors
/// - [`InvalidMetadata`] when `name` is missing or not a string, when `dependencies` is
///   missing or not an array, or when [`parse_dependency`] rejects an element. The reason names
///   the field.
fn parse_package(value: &Value) -> Result<Package, InvalidMetadata> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| InvalidMetadata {
            reason: "a package's `name` is missing or is not a string".to_owned(),
        })?;
    let dependencies = value
        .get("dependencies")
        .and_then(Value::as_array)
        .ok_or_else(|| InvalidMetadata {
            reason: format!("the `dependencies` of package `{name}` is missing or is not an array"),
        })?
        .iter()
        .map(parse_dependency)
        .collect::<Result<Vec<_>, InvalidMetadata>>()?;
    Ok(Package {
        name: name.to_owned(),
        kind: value
            .get("metadata")
            .and_then(|metadata| metadata.get("standards"))
            .and_then(|table| table.get("kind"))
            .and_then(Value::as_str)
            .and_then(to_crate_kind),
        dependencies,
    })
}

/// Reads one element of a package's `dependencies` array.
///
/// # Parameters
/// - `value`: the element. `name` is required. `kind` absent or `null` is a runtime
///   dependency, `"dev"` and `"build"` are those scopes.
///
/// # Errors
/// - [`InvalidMetadata`] when `name` is missing or not a string, or when `kind` is anything
///   else (another string, a number, a table). The reason names the field.
fn parse_dependency(value: &Value) -> Result<Dependency, InvalidMetadata> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| InvalidMetadata {
            reason: "a dependency's `name` is missing or is not a string".to_owned(),
        })?;
    let scope = match value.get("kind") {
        None | Some(Value::Null) => DependencyScope::Runtime,
        Some(Value::String(kind)) if kind == "dev" => DependencyScope::Dev,
        Some(Value::String(kind)) if kind == "build" => DependencyScope::Build,
        Some(other) => {
            return Err(InvalidMetadata {
                reason: format!(
                    "the `kind` of dependency `{name}` is not null, \"dev\" or \"build\": {other}"
                ),
            });
        }
    };
    Ok(Dependency {
        name: name.to_owned(),
        scope,
    })
}

/// Maps exactly `core`, `adapter`, `app` and `tool` to their kinds; anything else is `None`.
fn to_crate_kind(text: &str) -> Option<CrateKind> {
    match text {
        "core" => Some(CrateKind::Core),
        "adapter" => Some(CrateKind::Adapter),
        "app" => Some(CrateKind::App),
        "tool" => Some(CrateKind::Tool),
        _ => None,
    }
}

#[cfg(test)]
#[allow(
    clippy::too_many_lines,
    reason = "a test may spell out its whole expected value"
)]
#[path = "parse_metadata_tests.rs"]
mod tests;
