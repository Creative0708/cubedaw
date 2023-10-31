
pub trait Compat{
}

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(not(target_arch = "wasm32"))]
mod native;

pub fn create_platform_compat() -> Box<dyn Compat> {
    #[cfg(target_arch = "wasm32")]
    return Box::new(web::WebCompat);
    #[cfg(not(target_arch = "wasm32"))]
    return Box::new(native::NativeCompat);
}