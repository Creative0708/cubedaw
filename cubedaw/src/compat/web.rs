
use super::{CompatImpl, Compat};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/compat/web.js")]
extern {
    fn worker_init(num_workers: u32) -> JsValue;
    fn sendAudioJobs(job_data: &[u8]);
    fn initialize();
}

impl Compat {
    fn worker_init(num_workers: u32) -> JsValue { worker_init(num_workers) }
}

impl CompatImpl for Compat {
    fn send_audio_jobs(job_data: &[u8]) {
        sendAudioJobs(job_data)
    }
}

#[wasm_bindgen(js_name = "worker_init")]
pub fn worker_init_() -> JsValue{
    Compat::worker_init(1)
}

#[wasm_bindgen]
pub fn main() {
    use crate::TestApp;

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