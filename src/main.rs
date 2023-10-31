#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use cubedaw::TestApp;

    env_logger::init();

    eframe::run_native(
        "Hello, World!",
        eframe::NativeOptions {
            initial_window_size: Some(egui::Vec2 { x: 960.0, y: 540.0 }),
            ..Default::default()
        },
        Box::new(|cc| Box::new(TestApp::new(cc))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use cubedaw::TestApp;

    #[cfg(debug_assertions)]
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "app",
                eframe::WebOptions::default(),
                Box::new(|cc| Box::new(TestApp::new(cc))),
            ).await
            .expect("Failed to start web app");
    })
}