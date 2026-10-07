//! Record WebKitGTK child-process failures in the application log.

use webkit2gtk::{WebProcessTerminationReason, WebViewExt};

const LOG_TARGET: &str = "openfootmanager_lib::platform::web_process";

/// Attach before lengthy setup work, while the application's logger is already installed.
pub fn watch_web_processes(app: &tauri::App) {
    use tauri::Manager;
    for (label, window) in app.webview_windows() {
        let signal_label = label.clone();
        if let Err(error) = window.with_webview(move |view| {
            watch_webview(signal_label, &view.inner());
        }) {
            log::error!(target: LOG_TARGET, "[web-process] Could not observe WebKit webview '{label}': {error}");
        }
    }
}

fn watch_webview(label: String, view: &webkit2gtk::WebView) {
    view.connect_web_process_terminated(move |_, reason| {
        log_web_process_termination(&label, reason);
    });
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

    /// Given the production observer on a real WebKit view, when its native termination
    /// signal fires, then the application logger records its window and reason at Error.
    #[test]
    fn a_native_webkit_termination_signal_reaches_the_logger() {
        const CHILD: &str = "OFM_NATIVE_WEBKIT_SIGNAL_TEST";
        const SUCCESS: &str = "native WebKit termination reached the logger";
        // GTK owns a main thread. A fresh process and private display isolate it from the
        // parallel test harness and make this regression run on headless CI too.
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new("xvfb-run")
                .arg("--auto-servernum")
                .arg(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "platform::web_process::tests::a_native_webkit_termination_signal_reaches_the_logger",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .env("GDK_BACKEND", "x11")
                .env("GSETTINGS_BACKEND", "memory")
                .output()
                .expect("the native signal regression requires xvfb-run");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(output.status.success(), "{stdout}\n{stderr}");
            assert!(
                stdout.contains(SUCCESS),
                "native signal test did not run: {stdout}"
            );
            return;
        }

        use std::io::Write;
        use webkit2gtk::glib::prelude::ObjectExt;
        gtk::init().expect("the isolated display must initialize GTK");
        initialize_capture_logger();
        let view = webkit2gtk::WebView::new();
        watch_webview("native-signal-window".to_string(), &view);
        view.emit_by_name::<()>(
            "web-process-terminated",
            &[&WebProcessTerminationReason::ExceededMemoryLimit],
        );
        assert_logged_records(
            "native-signal-window",
            WebProcessTerminationReason::ExceededMemoryLimit,
        );
        drop(view);
        println!("{SUCCESS}");
        std::io::stdout().flush().unwrap();
        // Run process teardown on GTK's initializing thread, rather than libtest's main.
        std::process::exit(0);
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
