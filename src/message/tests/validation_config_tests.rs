//! Unit tests for validation configuration.

use crate::message::ports::validator::ValidationConfig;
use rstest::rstest;

#[rstest]
fn default_config_values() {
    let config = ValidationConfig::default();
    assert_eq!(config.max_message_size_bytes, 1024 * 1024);
    assert_eq!(config.max_content_parts, 100);
    assert_eq!(config.max_text_length, 100_000);
    assert!(!config.allow_empty_text);
}

#[rstest]
fn lenient_config_allows_empty_text() {
    let config = ValidationConfig::lenient();
    assert!(config.allow_empty_text);
}

#[rstest]
fn strict_config_has_reduced_limits() {
    let config = ValidationConfig::strict();
    assert_eq!(config.max_message_size_bytes, 256 * 1024);
    assert_eq!(config.max_content_parts, 20);
    assert_eq!(config.max_text_length, 10_000);
}
