use eeman::app::EemanApp;

fn main() {
    // Touch the global Tokio runtime so it's initialised before the GLib main
    // loop starts. Async work is dispatched with `RUNTIME.spawn`; results come
    // back on the main thread through `finish_on_main`.
    let _ = &*eeman::RUNTIME;

    eeman::fonts::register_fonts();

    let app = EemanApp::new();
    app.run();
}
