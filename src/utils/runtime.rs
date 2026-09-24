use std::sync::OnceLock;
use tokio::runtime::{Builder, Runtime};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Returns the shared tokio runtime used by the bluetooth scanner, the web
/// dashboard, and any other async subsystem.
///
/// On first call a small multi-threaded runtime is lazily created. Callers must
/// ensure the runtime is dropped (process exit) before the terminal is
/// restored, otherwise background tasks may still write to the tty.
pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
    })
}
