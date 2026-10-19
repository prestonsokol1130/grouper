//! Named background threads whose failures are always logged.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::thread::JoinHandle;

use crate::logging::panic_message;

/// How a thread started with [`spawn_logged`] finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadOutcome {
    /// The closure returned `Ok(())`.
    Completed,
    /// The closure returned `Err`; holds the error text.
    Failed(String),
    /// The closure panicked; holds the panic message.
    Panicked(String),
}

/// Spawns a thread called `name` that runs `f` and logs an `Err` or a panic.
///
/// The panic is caught inside the thread, so it never spreads to the caller.
/// Note: release builds use `panic = "abort"`, so there a panic is logged by
/// the panic hook and then ends the process instead of being caught.
pub fn spawn_logged<F>(name: &str, f: F) -> Result<JoinHandle<ThreadOutcome>, String>
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    let thread_name = name.to_owned();
    std::thread::Builder::new()
        .name(thread_name.clone())
        .spawn(move || match catch_unwind(AssertUnwindSafe(f)) {
            Ok(Ok(())) => ThreadOutcome::Completed,
            Ok(Err(error)) => {
                log::error!("thread {thread_name:?} failed: {error}");
                ThreadOutcome::Failed(error)
            }
            Err(payload) => {
                let message = panic_message(payload.as_ref());
                log::error!("thread {thread_name:?} panicked: {message}");
                ThreadOutcome::Panicked(message)
            }
        })
        .map_err(|e| format!("could not spawn thread {name:?}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survives_panicking_closure() {
        let handle = spawn_logged("test-panic", || panic!("boom")).unwrap();
        let outcome = handle.join().unwrap();
        assert_eq!(outcome, ThreadOutcome::Panicked("boom".into()));
    }

    #[test]
    fn reports_error() {
        let handle = spawn_logged("test-err", || Err("nope".into())).unwrap();
        assert_eq!(handle.join().unwrap(), ThreadOutcome::Failed("nope".into()));
    }

    #[test]
    fn reports_success_and_sets_name() {
        let handle = spawn_logged("test-ok", || {
            assert_eq!(std::thread::current().name(), Some("test-ok"));
            Ok(())
        })
        .unwrap();
        assert_eq!(handle.join().unwrap(), ThreadOutcome::Completed);
    }
}
