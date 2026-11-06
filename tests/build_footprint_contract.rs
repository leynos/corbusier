//! Contract test for the development build footprint rule.
//!
//! The Concordat `rust-dev-build-footprint` rule needs the default development
//! profile to disable incremental compilation and debug information, and a
//! dated nightly from 2026-07-31 to set the native Cargo metadata key. The
//! files are parsed, so a reformat or a moved comment cannot break the checks.

use toml::{Table, Value};

/// The manifest, the Cargo configuration and the footprint settings they hold.
const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
const CARGO_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/.cargo/config.toml"));

/// Parses a document; a parse failure is `None`, which no assertion accepts.
fn parse(text: &str) -> Option<Table> {
    text.parse().ok()
}

/// Looks up `table.key` in a parsed document.
fn lookup<'a>(document: &'a Table, table: &str, key: &str) -> Option<&'a Value> {
    document.get(table)?.as_table()?.get(key)
}

/// Looks up `[profile.dev].key`.
fn dev_profile<'a>(document: &'a Table, key: &str) -> Option<&'a Value> {
    document
        .get("profile")?
        .as_table()?
        .get("dev")?
        .as_table()?
        .get(key)
}

#[test]
fn the_development_profile_keeps_no_incremental_state_or_debug_information() {
    let manifest = parse(MANIFEST).unwrap_or_default();
    assert_eq!(
        dev_profile(&manifest, "incremental").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        dev_profile(&manifest, "debug").and_then(Value::as_integer),
        Some(0)
    );
}

#[test]
fn the_cargo_configuration_does_not_override_the_footprint_settings() {
    assert!(
        parse(CARGO_CONFIG).is_some(),
        "the Cargo configuration must parse"
    );
    let config = parse(CARGO_CONFIG).unwrap_or_default();
    assert!(dev_profile(&config, "incremental").is_none());
    assert!(dev_profile(&config, "debug").is_none());
}

#[test]
fn the_native_metadata_key_disables_embedding() {
    let config = parse(CARGO_CONFIG).unwrap_or_default();
    assert_eq!(
        lookup(&config, "unstable", "embed-metadata").and_then(Value::as_bool),
        Some(false)
    );
    assert!(lookup(&config, "unstable", "no-embed-metadata").is_none());
}
