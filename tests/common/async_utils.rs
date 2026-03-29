//! Shared async helpers for integration-test step modules.

/// Runs an async operation to completion, reusing the current runtime when one
/// is already active and otherwise creating a dedicated current-thread runtime.
pub fn run_async<T>(future: impl std::future::Future<Output = T>) -> Result<T, std::io::Error> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        return match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => {
                Ok(tokio::task::block_in_place(|| handle.block_on(future)))
            }
            tokio::runtime::RuntimeFlavor::CurrentThread => Err(std::io::Error::other(
                "cannot block_on within a current-thread Tokio runtime",
            )),
            _ => Err(std::io::Error::other(
                "unsupported Tokio runtime flavour for blocking async helper",
            )),
        };
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map(|runtime| runtime.block_on(future))
}
