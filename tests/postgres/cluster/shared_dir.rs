//! The install and data directories shared by every test process.
//!
//! The suite shares one cluster across its process-per-test cases, so the
//! directories are pinned here rather than left to the crate's per-process
//! default.

use std::ffi::OsString;

/// Returns the root shared by every test process of one user.
///
/// The user ID is part of the name so two users on one host never share, and
/// never fight over the ownership of, a cluster directory.
pub(super) fn shared_directory_root(temp_dir: &std::path::Path, uid: u32) -> std::path::PathBuf {
    temp_dir.join(format!("corbusier-pg-embed-{uid}"))
}

/// Pins one install and data directory under `root` for every test process.
///
/// This suite shares a single cluster across processes: the first process runs
/// `initdb`, and later ones find the initialized directory and only start the
/// server. `pg-embed-setup-unpriv` 0.6 gives each process a directory of its own
/// unless `PG_DATA_DIR` is set, which would make every test process run
/// `initdb` and leave a server behind for the next sweep. Setting the variables
/// restores the sharing. A data directory the caller already set wins and
/// nothing is pinned; a runtime directory the caller set is kept while the data
/// directory is pinned. The caller supplies `root` and the environment lookup so
/// the function touches neither the operating system nor the process
/// environment.
pub(super) fn shared_directory_changes(
    root: &std::path::Path,
    lookup: impl Fn(&str) -> Option<OsString>,
) -> Vec<(OsString, Option<OsString>)> {
    if lookup("PG_DATA_DIR").is_some() {
        return Vec::new();
    }
    let mut changes = vec![(
        OsString::from("PG_DATA_DIR"),
        Some(root.join("data").into_os_string()),
    )];
    if lookup("PG_RUNTIME_DIR").is_none() {
        changes.push((
            OsString::from("PG_RUNTIME_DIR"),
            Some(root.join("install").into_os_string()),
        ));
    }
    changes
}

#[cfg(test)]
mod tests {
    //! Tests for the pinned shared install and data directories.

    use super::{shared_directory_changes, shared_directory_root};
    use crate::postgres::cluster::env_utils::worker_env_changes;
    use crate::test_helpers::EnvVarGuard;
    use nix::unistd::Uid;
    use rstest::rstest;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    /// The root the entry point must pin to for the current user.
    fn expected_root() -> PathBuf {
        shared_directory_root(&std::env::temp_dir(), Uid::effective().as_raw())
    }

    /// The value `changes` sets for `key`, if it sets it.
    fn value_of(changes: &[(OsString, Option<OsString>)], key: &str) -> Option<OsString> {
        changes
            .iter()
            .find(|(name, _)| name == key)
            .and_then(|(_, value)| value.clone())
    }

    /// Renders each change as a `(name, value)` string pair for comparison.
    fn pinned(values: &[(OsString, Option<OsString>)]) -> Vec<(String, String)> {
        values
            .iter()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value
                        .as_ref()
                        .map(|v| v.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                )
            })
            .collect()
    }

    /// The root is one directory per user under the given temporary directory.
    #[rstest]
    #[case(1000, "/tmp/corbusier-pg-embed-1000")]
    #[case(0, "/tmp/corbusier-pg-embed-0")]
    fn the_root_is_named_for_the_user(#[case] uid: u32, #[case] expected: &str) {
        assert_eq!(
            shared_directory_root(Path::new("/tmp"), uid),
            Path::new(expected)
        );
    }

    /// Which variables the caller already set decides what is pinned, and the
    /// pinned values are the `data` and `install` children of one shared root.
    #[rstest]
    #[case::none_set(&[], &[("PG_DATA_DIR", "/r/data"), ("PG_RUNTIME_DIR", "/r/install")])]
    #[case::data_dir_set(&["PG_DATA_DIR"], &[])]
    #[case::runtime_dir_set(&["PG_RUNTIME_DIR"], &[("PG_DATA_DIR", "/r/data")])]
    #[case::both_set(&["PG_DATA_DIR", "PG_RUNTIME_DIR"], &[])]
    fn the_caller_s_variables_win(#[case] preset: &[&str], #[case] expected: &[(&str, &str)]) {
        let changes = shared_directory_changes(Path::new("/r"), |key| {
            preset.contains(&key).then(|| OsString::from("/elsewhere"))
        });
        let want: Vec<(String, String)> = expected
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        assert_eq!(pinned(&changes), want);
    }

    /// The entry point `worker_env_changes` composes the pinning with the real
    /// temporary directory and effective user: with neither variable set it
    /// returns both, rooted at `<temp dir>/corbusier-pg-embed-<uid>`, and with
    /// `PG_DATA_DIR` set it pins nothing. Skipped as root, where the entry point
    /// first needs the `pg_worker` binary.
    #[rstest]
    #[case::unset(false)]
    #[case::caller_set(true)]
    fn the_entry_point_pins_the_per_user_root_unless_the_caller_set_one(#[case] caller_set: bool) {
        if Uid::effective().is_root() {
            return;
        }
        let data = caller_set.then(|| OsString::from("/elsewhere/data"));
        let _env = EnvVarGuard::set_many(&[
            (OsString::from("PG_DATA_DIR"), data),
            (OsString::from("PG_RUNTIME_DIR"), None),
        ]);
        let (changes, _port_guard) = worker_env_changes().expect("the entry point succeeds");
        let root = expected_root();
        if caller_set {
            assert_eq!(value_of(&changes, "PG_DATA_DIR"), None);
            assert_eq!(value_of(&changes, "PG_RUNTIME_DIR"), None);
        } else {
            assert_eq!(
                value_of(&changes, "PG_DATA_DIR"),
                Some(root.join("data").into_os_string())
            );
            assert_eq!(
                value_of(&changes, "PG_RUNTIME_DIR"),
                Some(root.join("install").into_os_string())
            );
        }
    }

    /// The cluster the suite really starts, through `postgres_cluster`, uses the
    /// pinned data directory: the per-user shared one, unless the caller's own
    /// `PG_DATA_DIR` was set for the run.
    #[test]
    fn the_started_cluster_uses_the_pinned_data_directory() {
        let cluster = super::super::postgres_cluster().expect("the cluster starts");
        let expected = std::env::var_os("PG_DATA_DIR")
            .map_or_else(|| expected_root().join("data"), PathBuf::from);
        assert_eq!(cluster.bootstrap.settings.data_dir, expected);
    }
}
