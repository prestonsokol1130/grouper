//! `Grouper.exe --bench flyout`: a hidden measurement, not a feature.
//!
//! On a background thread it shows and hides the flyout [`RUNS`] times
//! through the normal show path, then logs one `flyout_bench` line with
//! the p50 and p95 of `flyout_show_us` (each show also logs its own line,
//! and the page logs `flyout_paint_ms`). It only shows and hides the
//! flyout: it never launches an app and never changes or saves the state.
//! It uses the first group, or, when there is none, a group made up in
//! memory from the first visible apps, which nothing ever saves.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, Runtime};

use super::hide::hide;
use super::window::{find_group, request_group};
use crate::state::{Group, Store};
use crate::util::spawn::spawn_logged;

/// How many times the flyout is shown.
pub const RUNS: usize = 100;

/// How long each show stays up: enough for the page to paint.
const SHOWN: Duration = Duration::from_millis(150);

/// The pause after each hide.
const GAP: Duration = Duration::from_millis(100);

/// How many apps a made-up group holds.
const MADE_UP_APPS: usize = 6;

/// A made-up group's id: never a real one (those are 10 characters).
const MADE_UP_ID: &str = "grouper-bench";

/// Set while a bench runs, so a second `--bench` does nothing.
static RUNNING: AtomicBool = AtomicBool::new(false);

/// Which group the bench shows.
#[derive(Debug, Clone)]
enum Target {
    /// A saved group, looked up on every show like a real click.
    Saved(String),
    /// A group that only exists in this bench.
    MadeUp(Group),
}

/// Starts the bench, unless one is already running.
pub fn start<R: Runtime>(app: &AppHandle<R>) {
    if RUNNING.swap(true, Ordering::SeqCst) {
        log::warn!("flyout bench is already running");
        return;
    }
    let handle = app.clone();
    let started = spawn_logged("flyout-bench", move || {
        let result = run(&handle);
        RUNNING.store(false, Ordering::SeqCst);
        result
    });
    if let Err(error) = started {
        RUNNING.store(false, Ordering::SeqCst);
        log::warn!("flyout bench: {error}");
    }
}

fn run<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let target = target(app)?;
    log::info!("flyout bench: {RUNS} shows of {target:?}");
    let mut times = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let shown = target.clone();
        if let Some(us) = on_main(app, move |app| show_once(app, &shown))? {
            times.push(us);
        }
        std::thread::sleep(SHOWN);
        on_main(app, |app| hide(app).unwrap_or_else(|e| log::warn!("{e}")))?;
        std::thread::sleep(GAP);
    }
    log::info!("{}", summary(&mut times));
    Ok(())
}

/// The first group, or a made-up one.
fn target<R: Runtime>(app: &AppHandle<R>) -> Result<Target, String> {
    let store = app
        .try_state::<Arc<Store>>()
        .ok_or("the app state is not loaded")?;
    Ok(store.read(|state| match state.groups.first() {
        Some(group) => Target::Saved(group.id.clone()),
        None => Target::MadeUp(Group {
            id: MADE_UP_ID.to_owned(),
            name: "Bench".to_owned(),
            app_ids: (state.apps.iter())
                .filter(|app| !app.hidden)
                .take(MADE_UP_APPS)
                .map(|app| app.id.clone())
                .collect(),
            ..Group::default()
        }),
    }))
}

/// One show, timed like a real `--flyout` request.
fn show_once<R: Runtime>(app: &AppHandle<R>, target: &Target) -> Option<u128> {
    let started = Instant::now();
    let group = match target {
        Target::Saved(id) => find_group(app, id)?,
        Target::MadeUp(group) => group.clone(),
    };
    request_group(app, &group, started)
}

/// Runs `work` on the main thread (where the windows live) and waits.
fn on_main<R: Runtime, T: Send + 'static>(
    app: &AppHandle<R>,
    work: impl FnOnce(&AppHandle<R>) -> T + Send + 'static,
) -> Result<T, String> {
    let (send, receive) = mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = send.send(work(&handle));
    })
    .map_err(|e| format!("could not reach the main thread: {e}"))?;
    receive
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "the main thread did not answer".to_owned())
}

/// The log line for the show times `times` (in microseconds).
fn summary(times: &mut [u128]) -> String {
    times.sort_unstable();
    let at = |p: usize| match times.len() {
        0 => 0,
        n => times
            .get((n * p).div_ceil(100).clamp(1, n) - 1)
            .copied()
            .unwrap_or_default(),
    };
    format!(
        "flyout_bench runs={RUNS} shown={} p50_us={} p95_us={} max_us={}",
        times.len(),
        at(50),
        at(95),
        at(100)
    )
}

#[cfg(test)]
mod tests {
    use super::{summary, RUNS};

    #[test]
    fn nearest_rank_percentiles() {
        let mut times: Vec<u128> = (1..=100).rev().collect();
        let line = summary(&mut times);
        assert_eq!(
            line,
            format!("flyout_bench runs={RUNS} shown=100 p50_us=50 p95_us=95 max_us=100")
        );
        let line = summary(&mut [7, 3]);
        assert!(
            line.ends_with("shown=2 p50_us=3 p95_us=7 max_us=7"),
            "{line}"
        );
        assert!(summary(&mut []).ends_with("shown=0 p50_us=0 p95_us=0 max_us=0"));
    }
}
