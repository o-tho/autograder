use eframe::egui::{Context, Ui};
use eframe::Frame;

pub struct CreateTemplate;

impl Default for CreateTemplate {
    fn default() -> Self {
        Self {}
    }
}

impl CreateTemplate {
    pub fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Creating a template is hard work ...");
        });
    }
}
