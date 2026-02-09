//! Utilities shared by worker helpers and tests.
//!
//! These helpers keep shell-escaping logic consistent between production
//! binaries and test wrappers.

/// Escapes a value for safe inclusion in a POSIX shell command.
///
/// Uses single-quote wrapping and the standard `'\''` sequence for embedded
/// quotes.
#[must_use]
pub fn shell_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            escaped.push_str("'\\''");
        } else {
            escaped.push(ch);
        }
    }
    escaped.push('\'');
    escaped
}
