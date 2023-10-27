
pub mod handler;

pub mod test;

pub trait Screen {
    fn get_id(&self) -> egui::Id;
    fn update(&mut self, state: &crate::Context, ui: &mut egui::Ui);
}
