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

#[cfg(test)]
mod tests {
    use super::shell_escape;

    #[test]
    fn shell_escape_handles_empty_string() {
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn shell_escape_preserves_whitespace() {
        assert_eq!(shell_escape("a b"), "'a b'");
    }

    #[test]
    fn shell_escape_escapes_single_quotes() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn shell_escape_preserves_unicode() {
        assert_eq!(shell_escape("éß漢"), "'éß漢'");
    }

    #[test]
    fn shell_escape_composes_safely_in_command() {
        let escaped = shell_escape("a b");
        let command = format!("printf %s {escaped}");
        assert_eq!(command, "printf %s 'a b'");
    }
}
