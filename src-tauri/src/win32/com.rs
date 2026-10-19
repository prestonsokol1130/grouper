//! Per-thread COM initialisation as an RAII guard.

use std::marker::PhantomData;

/// Keeps COM initialised on the current thread while it is alive.
///
/// Create one at the top of every thread that uses Shell or COM, and keep it
/// alive until every COM interface on that thread has been dropped. The guard
/// is neither `Send` nor `Sync`, because COM initialisation belongs to one
/// thread and must be undone on that same thread:
///
/// ```compile_fail,E0277
/// fn assert_send<T: Send>() {}
/// assert_send::<grouper_lib::win32::com::ComGuard>();
/// ```
///
/// Guards may be nested on one thread; each one undoes only its own
/// initialisation when dropped.
#[derive(Debug)]
pub struct ComGuard {
    /// True when this guard's `CoInitializeEx` call must be balanced by
    /// `CoUninitialize` on drop (it returned `S_OK` or `S_FALSE`).
    uninit_on_drop: bool,
    /// A raw pointer is neither `Send` nor `Sync`, so neither is the guard.
    _not_send: PhantomData<*const ()>,
}

impl ComGuard {
    /// Initialises COM on this thread as a single-threaded apartment (STA).
    ///
    /// - `S_OK` (first initialisation) and `S_FALSE` (already initialised as
    ///   STA) both succeed, and the guard calls `CoUninitialize` on drop.
    /// - `RPC_E_CHANGED_MODE` (the thread is already in another apartment,
    ///   usually the multithreaded one) also succeeds: COM is usable, but
    ///   this guard did not initialise it and will not uninitialise it.
    /// - Any other `HRESULT` is returned as `Err`.
    ///
    /// On non-Windows platforms this returns `Err("unsupported platform")`.
    pub fn new() -> Result<Self, String> {
        imp::initialize().map(|uninit_on_drop| Self {
            uninit_on_drop,
            _not_send: PhantomData,
        })
    }

    /// Whether dropping this guard calls `CoUninitialize`. False when the
    /// thread was already in a different apartment (`RPC_E_CHANGED_MODE`).
    pub fn uninitializes_on_drop(&self) -> bool {
        self.uninit_on_drop
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.uninit_on_drop {
            imp::uninitialize();
        }
    }
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

    /// Calls `CoInitializeEx`; `Ok(true)` means a `CoUninitialize` is owed.
    pub(super) fn initialize() -> Result<bool, String> {
        // SAFETY: the reserved argument is None (NULL) as documented, and
        // the matching CoUninitialize runs on this same thread because the
        // guard cannot leave it.
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if hr == RPC_E_CHANGED_MODE {
            return Ok(false);
        }
        if hr.is_ok() {
            return Ok(true);
        }
        Err(format!(
            "CoInitializeEx failed: {} ({:#010X})",
            hr.message(),
            hr.0
        ))
    }

    pub(super) fn uninitialize() {
        // SAFETY: only called from Drop for a guard whose CoInitializeEx
        // returned S_OK or S_FALSE on this thread.
        unsafe { CoUninitialize() }
    }
}

#[cfg(not(windows))]
mod imp {
    pub(super) fn initialize() -> Result<bool, String> {
        Err("unsupported platform".to_owned())
    }

    pub(super) fn uninitialize() {}
}

#[cfg(all(test, windows))]
mod tests {
    use super::ComGuard;

    #[test]
    fn changed_mode_succeeds_without_uninit() {
        use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
        let outcome = std::thread::spawn(|| {
            // SAFETY: balanced by the CoUninitialize below on this thread.
            let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            assert!(hr.is_ok());
            let guard = ComGuard::new().unwrap();
            assert!(!guard.uninitializes_on_drop());
            drop(guard);
            // SAFETY: balances the MTA CoInitializeEx above.
            unsafe { CoUninitialize() };
        })
        .join();
        assert!(outcome.is_ok());
    }
}
