//! A small logger for `--cleanup`, which runs without the Tauri app and so
//! without `tauri-plugin-log`. It appends to the same `grouper.log`.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use log::{LevelFilter, Log, Metadata, Record};

/// Appends each record to one open file.
struct FileLogger(Mutex<File>);

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or_default();
        if let Ok(mut file) = self.0.lock() {
            // A failed log write must not stop the cleanup.
            let _ = writeln!(
                file,
                "[unix {seconds}][cleanup][{}] {}",
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.0.lock() {
            let _ = file.flush();
        }
    }
}

/// Sends `log` records to `{log_dir}/grouper.log`. Any failure leaves
/// logging off; cleanup goes on without it.
pub fn install(log_dir: &Path) {
    if fs::create_dir_all(log_dir).is_err() {
        return;
    }
    let opened = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("grouper.log"));
    let Ok(file) = opened else {
        return;
    };
    let logger: &'static FileLogger = Box::leak(Box::new(FileLogger(Mutex::new(file))));
    if log::set_logger(logger).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}
