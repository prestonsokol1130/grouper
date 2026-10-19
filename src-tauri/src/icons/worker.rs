//! A fixed pool of background icon threads fed by one channel.
//!
//! Apps are queued in batches. Each thread holds its own
//! [`ComGuard`](crate::win32::com::ComGuard) and runs the pool's `process`
//! function on one app at a time; when every app of a batch is done, the
//! batch's `on_done` gets the ids that `process` reported.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

use crate::logging::panic_message;
use crate::state::AppEntry;
use crate::util::spawn::{spawn_logged, ThreadOutcome};
use crate::win32::com::ComGuard;

/// Threads in the app's icon pool.
pub const THREADS: usize = 3;

/// Works on one app; returns its id when the app should be reported.
pub type Process = Arc<dyn Fn(&AppEntry) -> Option<String> + Send + Sync>;

/// Called once with a finished batch's reported ids.
type OnDone = Box<dyn FnOnce(Vec<String>) + Send>;

/// One queued batch, shared by its jobs.
struct Batch {
    progress: Mutex<Progress>,
}

struct Progress {
    remaining: usize,
    ids: Vec<String>,
    on_done: Option<OnDone>,
}

impl Batch {
    /// Records one finished job. The last one calls `on_done` (unless
    /// `deliver` is false, as during shutdown).
    fn finish_one(&self, id: Option<String>, deliver: bool) {
        let finished = {
            let mut progress = self.progress.lock().unwrap_or_else(PoisonError::into_inner);
            progress.ids.extend(id);
            progress.remaining = progress.remaining.saturating_sub(1);
            if progress.remaining == 0 {
                let ids = std::mem::take(&mut progress.ids);
                progress.on_done.take().map(|on_done| (on_done, ids))
            } else {
                None
            }
        };
        // Called with the lock released, so on_done may take its time.
        if let Some((on_done, ids)) = finished {
            if deliver {
                on_done(ids);
            }
        }
    }
}

struct Job {
    app: AppEntry,
    batch: Arc<Batch>,
}

/// The pool. Dropping it (or calling [`IconWorker::shutdown`]) drops the
/// channel's only sender, which ends every thread once it is idle.
pub struct IconWorker {
    sender: Mutex<Option<Sender<Job>>>,
    stopping: Arc<AtomicBool>,
    handles: Mutex<Vec<JoinHandle<ThreadOutcome>>>,
}

impl IconWorker {
    /// Starts `threads` threads named `icon-worker-N` that run `process`.
    pub fn start(threads: usize, process: Process) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        let stopping = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::with_capacity(threads);
        for n in 0..threads {
            let receiver = Arc::clone(&receiver);
            let process = Arc::clone(&process);
            let stopping = Arc::clone(&stopping);
            // On failure `sender` is dropped, so threads already started end.
            handles.push(spawn_logged(&format!("icon-worker-{n}"), move || {
                run_thread(&receiver, &process, &stopping);
                Ok(())
            })?);
        }
        Ok(Self {
            sender: Mutex::new(Some(sender)),
            stopping,
            handles: Mutex::new(handles),
        })
    }

    /// Queues `apps` as one batch and returns at once. `on_done` runs on a
    /// pool thread after the last app is processed. An empty batch is
    /// ignored (`on_done` is not called).
    pub fn queue(
        &self,
        apps: Vec<AppEntry>,
        on_done: impl FnOnce(Vec<String>) + Send + 'static,
    ) -> Result<(), String> {
        if apps.is_empty() {
            return Ok(());
        }
        let sender = self.sender.lock().unwrap_or_else(PoisonError::into_inner);
        let sender = sender.as_ref().ok_or("the icon worker is shut down")?;
        let batch = Arc::new(Batch {
            progress: Mutex::new(Progress {
                remaining: apps.len(),
                ids: Vec::new(),
                on_done: Some(Box::new(on_done)),
            }),
        });
        for app in apps {
            let job = Job {
                app,
                batch: Arc::clone(&batch),
            };
            sender
                .send(job)
                .map_err(|_| "the icon worker threads have stopped".to_owned())?;
        }
        Ok(())
    }

    /// Stops the pool: jobs still queued are dropped without being run, and
    /// each thread ends after its current job. Returns the thread handles
    /// unjoined, so app exit never waits on a slow Shell call.
    pub fn shutdown(&self) -> Vec<JoinHandle<ThreadOutcome>> {
        self.stopping.store(true, Ordering::SeqCst);
        drop(
            self.sender
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take(),
        );
        std::mem::take(&mut *self.handles.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// One pool thread: takes jobs until every sender is gone.
fn run_thread(receiver: &Mutex<Receiver<Job>>, process: &Process, stopping: &AtomicBool) {
    // Shell icon extraction needs COM on this thread. Without it, extraction
    // fails and the fallback icon is used.
    let _com = ComGuard::new()
        .map_err(|error| log::warn!("icon worker has no COM: {error}"))
        .ok();
    loop {
        // The lock is released at the end of this statement.
        let next = receiver
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .recv();
        let Ok(job) = next else {
            break;
        };
        if stopping.load(Ordering::SeqCst) {
            job.batch.finish_one(None, false);
            continue;
        }
        // A panicking job counts as not written, and the batch still finishes.
        // Release builds use panic = "abort", so this only helps in debug and
        // test builds.
        let id = catch_unwind(AssertUnwindSafe(|| process(&job.app))).unwrap_or_else(|payload| {
            let message = panic_message(payload.as_ref());
            log::error!("icon job for app {} panicked: {message}", job.app.id);
            None
        });
        job.batch.finish_one(id, !stopping.load(Ordering::SeqCst));
    }
}
