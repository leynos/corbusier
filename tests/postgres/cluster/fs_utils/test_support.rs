//! Isolated installation fixtures and queries for UTC publication tests.

use super::{BoxError, open_ambient_dir, publish_utc_alias};
use camino::Utf8PathBuf;
use cap_std::fs_utf8::Dir;
use postgresql_embedded::Settings;
use rstest::fixture;
use std::collections::BTreeSet;
use std::io::{self, Read};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

/// Owns one isolated installation directory and removes it after the test.
pub(super) struct TemporaryInstall {
    parent: Dir,
    name: Utf8PathBuf,
    path: Utf8PathBuf,
}

impl TemporaryInstall {
    /// Creates the installation layout without adding either timezone file.
    fn new() -> Result<Self, BoxError> {
        let temp_path = Utf8PathBuf::from_path_buf(std::env::temp_dir()).map_err(|path| {
            io::Error::other(format!(
                "temporary directory is not UTF-8: {}",
                path.display()
            ))
        })?;
        let parent = open_ambient_dir(&temp_path)?;
        let name = Utf8PathBuf::from(format!(
            "corbusier-pg-timezone-{}-{}",
            std::process::id(),
            NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        parent.create_dir(&name)?;
        let installation = Self {
            path: temp_path.join(&name),
            parent,
            name,
        };
        installation.directory()?.create_dir_all("share/timezone")?;
        Ok(installation)
    }

    /// Opens the isolated installation through its existing parent capability.
    pub(super) fn directory(&self) -> io::Result<Dir> {
        self.parent.open_dir(&self.name)
    }

    /// Opens the directory that the `PostgreSQL` server uses for timezone files.
    pub(super) fn timezone_dir(&self) -> io::Result<Dir> {
        self.directory()?.open_dir("share/timezone")
    }

    /// Points `PostgreSQL` settings at the isolated installation.
    pub(super) fn settings(&self) -> Settings {
        Settings {
            installation_dir: self.path.clone().into_std_path_buf(),
            ..Settings::new()
        }
    }
}

impl Drop for TemporaryInstall {
    /// Removes only this test's installation, reporting failed cleanup.
    fn drop(&mut self) {
        if let Err(error) = self.parent.remove_dir_all(&self.name) {
            tracing::warn!(path = %self.path, %error, "failed to remove test installation");
        }
    }
}

/// Supplies an independent installation without mutating process environment.
#[fixture]
pub(super) fn installation() -> Result<TemporaryInstall, BoxError> {
    TemporaryInstall::new()
}

/// Lists every directory entry so assertions also detect leaked staging files.
pub(super) fn timezone_files(directory: &Dir) -> io::Result<BTreeSet<String>> {
    directory
        .entries()?
        .map(|entry| entry?.file_name())
        .collect()
}

/// Supplies a permission template for synthetic publication-test readers.
pub(super) fn publish_test_source(directory: &Dir, source: &mut impl Read) -> io::Result<()> {
    publish_utc_alias(directory, source, directory.dir_metadata()?.permissions())
}
