//! Generated coverage of UTC publication contents, failures, and competing writers.

use super::test_support::{installation, publish_test_source, timezone_files};
use proptest::collection::vec;
use proptest::prelude::{any, prop_assert, prop_assert_eq, proptest};
use proptest::test_runner::TestCaseError;
use std::collections::BTreeSet;
use std::io::{self, Cursor, Read, Take};
use std::sync::{Arc, Barrier};
use std::thread;

/// Supplies a generated prefix, then either EOF or a deterministic copy error.
struct PublicationSource {
    prefix: Take<Cursor<Vec<u8>>>,
    should_fail: bool,
}

impl Read for PublicationSource {
    /// Injects failure only after the generated prefix has reached the stage.
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let copied = self.prefix.read(buffer)?;
        if copied == 0 && self.should_fail {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "injected copy failure",
            ))
        } else {
            Ok(copied)
        }
    }
}

proptest! {
    /// Preserves the first published bytes across arbitrary later repair attempts.
    #[test]
    fn published_alias_is_idempotent(
        original in proptest::option::of(vec(any::<u8>(), 0..4097)),
        payload in vec(any::<u8>(), 0..4097),
        replacement in vec(any::<u8>(), 0..4097),
    ) {
        let isolated = installation().map_err(|error| TestCaseError::fail(error.to_string()))?;
        let directory = isolated.timezone_dir()?;
        if let Some(bytes) = &original {
            directory.write("UTC", bytes)?;
        }

        publish_test_source(&directory, &mut Cursor::new(&payload))?;
        publish_test_source(&directory, &mut Cursor::new(replacement))?;

        let expected = original.as_ref().unwrap_or(&payload);
        let published = directory.read("UTC")?;
        prop_assert_eq!(&published, expected);
        prop_assert_eq!(timezone_files(&directory)?, BTreeSet::from(["UTC".to_owned()]));
    }

    /// Keeps partial copies private and removes their stages at generated failure offsets.
    #[test]
    fn interrupted_publication_preserves_previous_alias(
        original in proptest::option::of(vec(any::<u8>(), 0..4097)),
        payload in vec(any::<u8>(), 0..4097),
        failure_after in 0u64..4097,
    ) {
        let isolated = installation().map_err(|error| TestCaseError::fail(error.to_string()))?;
        let directory = isolated.timezone_dir()?;
        if let Some(bytes) = &original {
            directory.write("UTC", bytes)?;
        }
        let mut source = PublicationSource {
            prefix: Cursor::new(payload).take(failure_after),
            should_fail: true,
        };

        let outcome = publish_test_source(&directory, &mut source);

        prop_assert_eq!(
            outcome.err().map(|error| error.kind()),
            Some(io::ErrorKind::UnexpectedEof),
        );
        let expected_files = if let Some(bytes) = original {
            prop_assert_eq!(directory.read("UTC")?, bytes);
            BTreeSet::from(["UTC".to_owned()])
        } else {
            prop_assert_eq!(
                directory.metadata("UTC").err().map(|error| error.kind()),
                Some(io::ErrorKind::NotFound),
            );
            BTreeSet::new()
        };
        prop_assert_eq!(timezone_files(&directory)?, expected_files);
    }

    /// Allows only complete successful sources to win among generated competing attempts.
    #[test]
    fn competing_publications_leave_a_complete_winner(
        attempts in vec((vec(any::<u8>(), 0..1025), any::<bool>(), 0u64..1025), 1..5),
    ) {
        let isolated = installation().map_err(|error| TestCaseError::fail(error.to_string()))?;
        let directory = Arc::new(isolated.timezone_dir()?);
        let barrier = Arc::new(Barrier::new(attempts.len()));
        thread::scope(|scope| {
            let handles: Vec<_> = attempts.iter().map(|(payload, should_fail, failure_after)| {
                let worker_directory = Arc::clone(&directory);
                let worker_barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    let copy_limit = if *should_fail { *failure_after } else { u64::MAX };
                    let mut source = PublicationSource {
                        prefix: Cursor::new(payload.clone()).take(copy_limit),
                        should_fail: *should_fail,
                    };
                    worker_barrier.wait();
                    let outcome = publish_test_source(&worker_directory, &mut source);
                    let expected_error = should_fail.then_some(io::ErrorKind::UnexpectedEof);
                    if outcome.err().map(|error| error.kind()) != expected_error {
                        return Err(io::Error::other("publication returned an unexpected outcome"));
                    }
                    Ok(())
                })
            }).collect();
            for handle in handles {
                handle
                    .join()
                    .map_err(|_| io::Error::other("publication thread panicked"))??;
            }
            Ok::<(), io::Error>(())
        })?;

        let successful_payloads: Vec<_> = attempts.iter()
            .filter(|(_, should_fail, _)| !should_fail)
            .map(|(payload, _, _)| payload)
            .collect();
        let expected_files = if successful_payloads.is_empty() {
            prop_assert_eq!(
                directory.metadata("UTC").err().map(|error| error.kind()),
                Some(io::ErrorKind::NotFound),
            );
            BTreeSet::new()
        } else {
            let published = directory.read("UTC")?;
            prop_assert!(successful_payloads.contains(&&published));
            BTreeSet::from(["UTC".to_owned()])
        };
        prop_assert_eq!(timezone_files(&directory)?, expected_files);
    }
}
