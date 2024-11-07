use eframe::egui::{Context, Ui};
use eframe::Frame;

pub struct Help;

impl Default for Help {
    fn default() -> Self {
        Self {}
    }
}

impl Help {
    pub fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Here I'll talk about documentation");
        });
    }
}
