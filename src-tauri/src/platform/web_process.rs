//! Record WebKitGTK child-process failures in the application log.

use webkit2gtk::{WebProcessTerminationReason, WebViewExt};

const LOG_TARGET: &str = "openfootmanager_lib::platform::web_process";

/// Attach before lengthy setup work, while the application's logger is already installed.
pub fn watch_web_processes(app: &tauri::App) {
    use tauri::Manager;
    for (label, window) in app.webview_windows() {
        let signal_label = label.clone();
        if let Err(error) = window.with_webview(move |view| {
            register_termination_observer(signal_label, |callback| {
                view.inner()
                    .connect_web_process_terminated(move |_, reason| callback(reason));
            });
        }) {
            log::error!(target: LOG_TARGET, "[web-process] Could not observe WebKit webview '{label}': {error}");
        }
    }
}

type TerminationCallback = Box<dyn Fn(WebProcessTerminationReason)>;

fn register_termination_observer(label: String, connect: impl FnOnce(TerminationCallback)) {
    connect(Box::new(move |reason| {
        log_web_process_termination(&label, reason);
    }));
}

fn log_web_process_termination(label: &str, reason: WebProcessTerminationReason) {
    log::error!(target: LOG_TARGET, "[web-process] WebKit webview '{label}' web process terminated: {reason:?}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    struct CaptureLogger(Mutex<Vec<(log::Level, String)>>);

    impl log::Log for CaptureLogger {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.target() == LOG_TARGET
        }

        fn log(&self, record: &log::Record<'_>) {
            if self.enabled(record.metadata()) {
                self.0
                    .lock()
                    .unwrap()
                    .push((record.level(), record.args().to_string()));
            }
        }

        fn flush(&self) {}
    }

    static LOGGER: CaptureLogger = CaptureLogger(Mutex::new(Vec::new()));
    static LOGGER_READY: OnceLock<()> = OnceLock::new();

    fn initialize_capture_logger() {
        LOGGER_READY.get_or_init(|| {
            log::set_logger(&LOGGER).unwrap();
            log::set_max_level(log::LevelFilter::Info);
        });
    }

    fn assert_logged_records(label: &str, reason: WebProcessTerminationReason) {
        let entries: Vec<_> = {
            let records = LOGGER.0.lock().unwrap();
            records
                .iter()
                .filter(|(_, message)| message.contains(label))
                .cloned()
                .collect()
        };
        assert_eq!(
            entries.len(),
            1,
            "one termination event must produce one log entry"
        );
        assert_eq!(entries[0].0, log::Level::Error);
        assert!(entries[0].1.contains(&format!("{reason:?}")));
    }

    fn assert_error_logged(label: &str, reason: WebProcessTerminationReason) {
        initialize_capture_logger();
        log_web_process_termination(label, reason);
        assert_logged_records(label, reason);
    }

    /// Given a webview signal connector, when its registered termination callback fires,
    /// then that callback reaches the application logger at Error with its window and reason.
    #[test]
    fn a_registered_web_process_termination_callback_reaches_the_logger() {
        initialize_capture_logger();
        let mut callback = None;
        register_termination_observer("registered-window".to_string(), |registered| {
            callback = Some(registered);
        });
        callback.expect("a termination observer must be registered")(
            WebProcessTerminationReason::Crashed,
        );
        assert_logged_records("registered-window", WebProcessTerminationReason::Crashed);
    }

    /// Given WebKit crashes, when its termination is observed, then the label and cause are logged at Error.
    #[test]
    fn a_crashed_web_process_is_logged_at_error() {
        assert_error_logged("crash-window", WebProcessTerminationReason::Crashed);
    }

    /// Given WebKit exceeds its memory limit, when it terminates, then the failure is logged at Error.
    #[test]
    fn a_web_process_memory_limit_is_logged_at_error() {
        assert_error_logged(
            "memory-window",
            WebProcessTerminationReason::ExceededMemoryLimit,
        );
    }

    /// Given WebKit is terminated by API, when it emits the signal, then its cause is logged at Error.
    #[test]
    fn an_api_terminated_web_process_is_logged_at_error() {
        assert_error_logged("api-window", WebProcessTerminationReason::TerminatedByApi);
    }

    /// Given a future WebKit reason, when the process terminates, then its numeric reason is retained at Error.
    #[test]
    fn an_unknown_web_process_termination_is_logged_at_error() {
        assert_error_logged("unknown-window", WebProcessTerminationReason::__Unknown(42));
    }
}
