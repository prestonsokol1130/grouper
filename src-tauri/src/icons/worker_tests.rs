//! Tests for the icon worker pool, with a fake `process` function.

use std::collections::{BTreeSet, HashSet};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::worker::{IconWorker, Process, THREADS};
use crate::state::AppEntry;
use crate::util::spawn::ThreadOutcome;

fn apps(ids: &[&str]) -> Vec<AppEntry> {
    ids.iter()
        .map(|id| AppEntry {
            id: (*id).into(),
            ..AppEntry::default()
        })
        .collect()
}

/// Reports every app except those whose id starts with `skip`, and records
/// which thread ran each one.
fn reporting_process(threads: Arc<Mutex<HashSet<String>>>) -> Process {
    Arc::new(move |app: &AppEntry| {
        let name = std::thread::current().name().unwrap_or("").to_owned();
        threads.lock().unwrap().insert(name);
        std::thread::sleep(Duration::from_millis(5));
        (!app.id.starts_with("skip")).then(|| app.id.clone())
    })
}

#[test]
fn processes_a_queue_and_reports_the_ids() {
    let threads = Arc::new(Mutex::new(HashSet::new()));
    let worker = IconWorker::start(THREADS, reporting_process(Arc::clone(&threads))).unwrap();
    let (done, reported) = mpsc::channel();
    let ids = ["a", "b", "skip-c", "d", "e", "skip-f", "g", "h", "i"];
    worker
        .queue(apps(&ids), move |ids| done.send(ids).unwrap())
        .unwrap();

    let got: BTreeSet<String> = reported
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .into_iter()
        .collect();
    let want: BTreeSet<String> = ["a", "b", "d", "e", "g", "h", "i"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(got, want);
    // Exactly one report per batch.
    assert!(reported.recv_timeout(Duration::from_millis(100)).is_err());
    assert!(threads
        .lock()
        .unwrap()
        .iter()
        .all(|name| name.starts_with("icon-worker-")));

    for handle in worker.shutdown() {
        assert_eq!(handle.join().unwrap(), ThreadOutcome::Completed);
    }
}

#[test]
fn each_batch_reports_on_its_own() {
    let worker = IconWorker::start(2, reporting_process(Arc::default())).unwrap();
    let (done, reported) = mpsc::channel();
    let second = done.clone();
    worker
        .queue(apps(&["a1", "a2"]), move |ids| done.send(ids).unwrap())
        .unwrap();
    worker
        .queue(apps(&["b1"]), move |ids| second.send(ids).unwrap())
        .unwrap();
    let mut batches = vec![
        reported.recv_timeout(Duration::from_secs(10)).unwrap(),
        reported.recv_timeout(Duration::from_secs(10)).unwrap(),
    ];
    batches.iter_mut().for_each(|ids| ids.sort());
    batches.sort();
    assert_eq!(batches, vec![vec!["a1", "a2"], vec!["b1"]]);
    drop(worker.shutdown());
}

#[test]
fn an_empty_batch_is_ignored() {
    let worker = IconWorker::start(1, reporting_process(Arc::default())).unwrap();
    let (done, reported) = mpsc::channel::<Vec<String>>();
    worker
        .queue(Vec::new(), move |ids| done.send(ids).unwrap())
        .unwrap();
    assert!(reported.recv_timeout(Duration::from_millis(100)).is_err());
    drop(worker.shutdown());
}

#[test]
fn shutdown_ends_the_threads_and_refuses_new_work() {
    let worker = IconWorker::start(THREADS, reporting_process(Arc::default())).unwrap();
    let handles = worker.shutdown();
    assert_eq!(handles.len(), THREADS);
    for handle in handles {
        assert_eq!(handle.join().unwrap(), ThreadOutcome::Completed);
    }
    assert!(worker.queue(apps(&["late"]), |_| {}).is_err());
}
