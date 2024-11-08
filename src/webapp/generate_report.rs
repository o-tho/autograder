use crate::image_container::{ImageContainer, PdfContainer, SingleImageContainer, TiffContainer};
use crate::image_helpers::rgb_to_egui_color_image;
use crate::report::{create_zip_from_imagereports, ImageReport};
use crate::scan::Scan;
use crate::template::{ExamKey, Template};
use crate::webapp::utils::{download_button, upload_button, FileType};
use egui::Context;
use infer;
use rayon::prelude::*;
use serde_json;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct GenerateReport {
    template: Option<Template>,
    key: Option<ExamKey>,
    container: Option<Arc<dyn ImageContainer>>,
    zipped_results: Option<Vec<u8>>,
    data_channel: (Sender<(FileType, Vec<u8>)>, Receiver<(FileType, Vec<u8>)>),
    preview_texture: Option<egui::TextureHandle>,
}

impl Default for GenerateReport {
    fn default() -> Self {
        let (sender, receiver) = channel(50);
        Self {
            template: None,
            key: None,
            container: None,
            data_channel: (sender, receiver),
            zipped_results: None,
            preview_texture: None,
        }
    }
}

impl eframe::App for GenerateReport {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(5, |columns| {
                columns[0].vertical(|ui| {
                    ui.horizontal(|ui| {
                        upload_button(
                            ui,
                            &ctx,
                            "📂 Upload Template",
                            FileType::Template,
                            self.data_channel.0.clone(),
                        );
                        if self.template.is_some() {
                            ui.label("🎉️");
                        }
                    });
                    ui.label("Upload a template file (.json).");
                });
                columns[1].vertical(|ui| {
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
                    ui.label("Upload an exam key (.json).");
                });
                columns[2].vertical(|ui| {
                    ui.horizontal(|ui| {
                        upload_button(
                            ui,
                            &ctx,
                            "📂 Upload Container",
                            FileType::Container,
                            self.data_channel.0.clone(),
                        );
                        if self.container.is_some() {
                            ui.label("❤");
                        }
                    });
                    ui.label("Upload an image container (.pdf, .tiff, .jpg, .png)");
                });
                columns[3].vertical(|ui| {
                    if self.template.is_some() && self.key.is_some() && self.container.is_some() {
                        if ui.button("🚀 Do the thing!").clicked() {
                            log::info!("Zhu Li! Get the container!");
                            if let Some(arc_container) = &mut self.container {
                                if let Some(container) = Arc::get_mut(arc_container) {
                                    log::info!("Unpack it!");
                                    match container.to_vector() {
                                        Ok(vector) => {
                                            let template = self.template.clone().unwrap();
                                            let key = self.key.clone().unwrap();
                                            log::info!("Generate the image reports!");
                                            let res: Vec<ImageReport> = vector
                                                .par_iter()
                                                .enumerate()
                                                .map(|i| {
                                                    log::info!("Generating a report ...");
                                                    let mut scan = Scan {
                                                        img: i.1.clone(),
                                                        transformation: None,
                                                    };
                                                    scan.transformation =
                                                        scan.find_transformation(&template);
                                                    scan.generate_imagereport(
                                                        &template,
                                                        &key,
                                                        &format!("page{}", i.0),
                                                    )
                                                })
                                                .collect();

                                            log::info!("Create a zip file!");
                                            let zipfile =
                                                create_zip_from_imagereports(&res).unwrap();
                                            self.zipped_results = Some(zipfile);

                                            log::info!("Display a preview!");
                                            let result = &res[0];

                                            let display = rgb_to_egui_color_image(&result.image);
                                            self.preview_texture = Some(ui.ctx().load_texture(
                                                "displayed_image",
                                                display,
                                                egui::TextureOptions::default(),
                                            ));
                                            log::info!("Zhu Li! You have done the thing!");
                                        }
                                        Err(err) => {
                                            log::error!("Error calling to_vector: {:?}", err);
                                        }
                                    }
                                }
                            }
                        }
                    }
                });

                columns[4].vertical(|ui| {
                    if let Some(zipfile) = self.zipped_results.clone() {
                        download_button(ui, "💾 Save results as zip file", zipfile);
                    }
                });
            });
            // Conditionally display the computation button when all files are uploaded
            if let Some(texture) = &self.preview_texture {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add(egui::Image::new(texture));
                });
            }

            // Handle incoming file data and deserialize as needed
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
                    FileType::Container => {
                        if let Some(kind) = infer::get(&data) {
                            match kind.mime_type() {
                                "application/pdf" => {
                                    let pdf =
                                        pdf::file::FileOptions::uncached().load(data).unwrap();
                                    let container = PdfContainer { pdf_file: pdf };

                                    self.container = Some(Arc::new(container));
                                }
                                "image/tiff" => {
                                    let buffer = std::io::Cursor::new(data);
                                    let tiff = tiff::decoder::Decoder::new(buffer).unwrap();
                                    let container = TiffContainer { decoder: tiff };
                                    self.container = Some(Arc::new(container));
                                }
                                "image/jpeg" => {
                                    let image = image::load_from_memory_with_format(
                                        &data,
                                        image::ImageFormat::Jpeg,
                                    )
                                    .unwrap();
                                    let container = SingleImageContainer { image: image };
                                    self.container = Some(Arc::new(container));
                                }
                                _ => log::error!(
                                    "{}",
                                    format!("Unsupported container format {}", kind.mime_type())
                                ),
                            }
                        } else {
                            log::error!("Could not infer file type");
                        }
                    }
                    _ => {}
                }
            }
        });
    }
}
