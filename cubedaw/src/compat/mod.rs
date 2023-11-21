
pub struct Compat{}

pub trait CompatImpl{
    fn send_audio_jobs(job_data: &[u8]);
}

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub mod prelude {
    pub use super::CompatImpl;
    pub use super::web::*;
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub mod prelude {
    pub use super::CompatImpl;
    pub use super::native::*;
}