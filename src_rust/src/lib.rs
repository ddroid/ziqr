pub mod app;
pub mod config;
pub mod fonts;
pub mod main_window;
pub mod preferences;
pub mod prayer;
pub mod quran;
pub mod welcome;

// Global Tokio runtime for async HTTP requests. The GLib main loop runs the UI
// on the main thread; this runtime's worker threads drive `reqwest` futures.
// Results come back through a oneshot that the main loop waits on, so the UI
// thread is not polling.
//
// The runtime must have `rt-multi-thread` enabled so `tokio::task::spawn_blocking`
// (used internally by hyper for DNS over `getaddrinfo`) has a destination worker.
lazy_static::lazy_static! {
    pub static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");
}

/// Run `future` on the Tokio runtime and call `done` on the GTK main thread.
///
/// `done` waits on a oneshot. That parks the main-loop task until the fetch
/// finishes, instead of rescheduling an idle callback on every loop turn.
pub fn finish_on_main<T, F>(future: impl std::future::Future<Output = T> + Send + 'static, done: F)
where
    T: Send + 'static,
    F: FnOnce(T) + 'static,
{
    let (tx, rx) = futures::channel::oneshot::channel();
    RUNTIME.spawn(async move {
        let _ = tx.send(future.await);
    });
    glib::spawn_future_local(async move {
        if let Ok(value) = rx.await {
            done(value);
        }
    });
}
