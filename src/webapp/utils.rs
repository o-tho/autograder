use eframe::egui::{Context, Ui};
use std::future::Future;
use tokio::sync::mpsc::Sender;

pub enum FileType {
    Template,
    Key,
    Container,
}
pub fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

pub fn upload_button(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    label: &str,
    file_type: FileType,
    sender: Sender<(FileType, Vec<u8>)>,
) {
    if ui.button(label).clicked() {
        let sender = sender.clone();
        let ctx = ctx.clone();
        let task = rfd::AsyncFileDialog::new().pick_file();
        execute(async move {
            if let Some(file) = task.await {
                let bytes = file.read().await;
                if let Err(e) = sender.send((file_type, bytes)).await {
                    log::error!("Failed to send file data: {}", e);
                } else {
                    log::info!("File data sent successfully");
                }
                ctx.request_repaint();
            }
        });
    }
}

pub fn download_button(ui: &mut egui::Ui, label: &str, data: Vec<u8>) {
    if ui.button(label).clicked() {
        let task = rfd::AsyncFileDialog::new().save_file();
        execute(async move {
            let file = task.await;
            if let Some(file) = file {
                _ = file.write(&data).await;
            }
        });
    }
}
