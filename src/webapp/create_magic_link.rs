use crate::template::{ExamKey, Template};
use crate::webapp::utils::{encode_key_template, upload_button, FileType};
use eframe::egui::{Context, ScrollArea};
use eframe::Frame;

use tokio::sync::mpsc::{channel, Receiver, Sender};
pub struct CreateMagicLink {
    template: Option<Template>,
    key: Option<ExamKey>,
    data_channel: (Sender<(FileType, Vec<u8>)>, Receiver<(FileType, Vec<u8>)>),
}

impl Default for CreateMagicLink {
    fn default() -> Self {
        Self {
            template: None,
            key: None,
            data_channel: channel(50),
        }
    }
}

impl CreateMagicLink {
    fn to_link(&self) -> String {
        let location = web_sys::window().expect("we need a window").location();

        if let (Some(key), Some(template)) = (self.key.clone(), self.template.clone()) {
            format!(
                "{}{}#{}",
                location.origin().unwrap_or("".into()),
                location.pathname().unwrap_or("".into()),
                encode_key_template(&key, &template)
            )
        } else {
            "".into()
        }
    }
}

impl CreateMagicLink {
    pub fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("A ✨magic link✨ allows you to share a template and an exam key together as a single link. This is great if you want to use autograder for specific exams and have to potentially re-grade many exams without having to upload the template and key file each time. Best to bookmark!");
            ui.horizontal(|ui| {
                upload_button(
                    ui,
                    &ctx,
                    "📂 Upload Template",
                    FileType::Template,
                    self.data_channel.0.clone(),
                );
                if self.template.is_some() {
                    ui.label("🎉");
                }
            });

            ui.horizontal(|ui| {
                upload_button(
                    ui,
                    &ctx,
                    "📂 Upload Exam Key",
                    FileType::Key,
                    self.data_channel.0.clone(),
                );
                if self.key.is_some() {
                    ui.label("👍");
                }
            });
            if self.template.is_some() && self.key.is_some() {
                let link = self.to_link();
                ui.hyperlink_to("This is your magic link ✨", link.clone());
                ScrollArea::vertical().show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut link.as_str()));
                });
            }
        });
        while let Ok((file_type, data)) = self.data_channel.1.try_recv() {
            match file_type {
                FileType::Template => {
                    if let Ok(template) = serde_json::from_slice::<Template>(&data) {
                        self.template = Some(template);
                        log::info!("loaded template");
                    } else {
                        log::error!("could not parse template");
                    }
                }
                FileType::Key => {
                    if let Ok(key) = serde_json::from_slice::<ExamKey>(&data) {
                        self.key = Some(key);
                        log::info!("loaded key");
                    } else {
                        log::error!("could not parse template");
                    }
                }
                _ => {}
            }
        }
    }
}