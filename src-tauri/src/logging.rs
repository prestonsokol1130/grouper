//! Log plugin setup and the panic safety net.

use log::LevelFilter;
use tauri::plugin::TauriPlugin;
use tauri::Runtime;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

/// Base name of the log file inside the app log directory (`grouper.log`).
const LOG_FILE_NAME: &str = "grouper";
/// Rotate the log once it reaches 2 MB.
const MAX_LOG_BYTES: u128 = 2 * 1024 * 1024;
/// Number of rotated log files to keep.
const KEEP_LOG_FILES: usize = 3;

/// Builds the log plugin: file in the app log dir, plus stdout and the
/// webview console in debug builds. Info level in release, Debug in debug.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    let file = Target::new(TargetKind::LogDir {
        file_name: Some(LOG_FILE_NAME.into()),
    });
    let builder = tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(file)
        .max_file_size(MAX_LOG_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(KEEP_LOG_FILES));
    let builder = if cfg!(debug_assertions) {
        builder
            .level(LevelFilter::Debug)
            .target(Target::new(TargetKind::Stdout))
            .target(Target::new(TargetKind::Webview))
    } else {
        builder.level(LevelFilter::Info)
    };
    builder.build()
}

/// Logs every panic (message and location) before the default hook runs.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".into());
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("unnamed");
        log::error!(
            "panic in thread {name:?} at {location}: {}",
            panic_message(info.payload())
        );
        default_hook(info);
    }));
}

/// Extracts the text from a panic payload when it is a string.
pub fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "non-string panic payload".into()
    }
}

#[cfg(test)]
mod tests {
    use super::panic_message;

    #[test]
    fn panic_message_reads_str_and_string() {
        let a: Box<dyn std::any::Any + Send> = Box::new("boom");
        let b: Box<dyn std::any::Any + Send> = Box::new(String::from("bang"));
        let c: Box<dyn std::any::Any + Send> = Box::new(7_u8);
        assert_eq!(panic_message(a.as_ref()), "boom");
        assert_eq!(panic_message(b.as_ref()), "bang");
        assert_eq!(panic_message(c.as_ref()), "non-string panic payload");
    }
}
