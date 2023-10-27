
pub trait Compat{
}

mod web;

pub fn create_platform_compat() -> Box<dyn Compat> {
    return Box::new(web::WebCompat);
}