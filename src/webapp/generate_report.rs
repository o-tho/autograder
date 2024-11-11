use crate::image_container::{ImageContainer, PdfContainer, SingleImageContainer, TiffContainer};
use crate::image_helpers::rgb_to_egui_color_image;
use crate::report::ImageReport;
use crate::scan::Scan;
use crate::template::{ExamKey, Template};
use crate::webapp::utils::{download_button, upload_button, FileType};
use egui::Context;
use infer;
use itertools::Itertools;
use poll_promise::Promise;
use rayon::prelude::*;
use serde_json;
use std::io::Cursor;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use zip::write::FileOptions;
use zip::ZipWriter;

pub struct GenerateReport {
    template: Option<Template>,
    key: Option<ExamKey>,
    container: Option<Arc<Mutex<dyn ImageContainer + Send + Sync>>>,
    zipped_results: Option<Promise<Option<Vec<u8>>>>,
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

impl GenerateReport {
    pub fn generate_reports(&mut self) {
        let template = self.template.clone().unwrap();
        let key = self.key.clone().unwrap();

        if let Some(container) = self.container.clone() {
            let promise = Promise::spawn_local(async move {
                let mut zip_buffer = Cursor::new(Vec::new());
                let mut zip_writer = ZipWriter::new(&mut zip_buffer);
                let mut csv_writer = csv::Writer::from_writer(std::io::Cursor::new(Vec::new()));
                let _ = csv_writer.write_record(["Filename", "ID", "Score"]);

                log::info!("Output files are set up, starting to iterate over the input images!");

                let mut container_lock = container.lock().unwrap();
                let iterator = container_lock.to_iter();

                let mut turn = 0;
                let chunksize = 100;

                log::info!(
                    "Working on images {}-{}",
                    turn * chunksize,
                    (turn + 1) * chunksize
                );
                for chunk in &iterator.chunks(chunksize) {
                    let images: Vec<image::GrayImage> = chunk.collect();
                    let results: Vec<ImageReport> = images
                        .par_iter()
                        .enumerate()
                        .map(|(idx, img)| {
                            log::info!("processing {}", turn * chunksize + idx);
                            let mut scan = Scan {
                                img: img.clone(),
                                transformation: None,
                            };
                            scan.transformation = scan.find_transformation(&template);
                            scan.generate_imagereport(
                                &template,
                                &key,
                                &format!("page{}", idx + turn * chunksize),
                            )
                        })
                        .collect();

                    for r in &results {
                        let _ = r.add_to_zip(&mut zip_writer, &mut csv_writer);
                    }

                    // let display = rgb_to_egui_color_image(&results[0].image);
                    // self.preview_texture = Some(ui.ctx().load_texture(
                    //     "displayed_image",
                    //     display,
                    //     egui::TextureOptions::default(),
                    // ));

                    turn += 1;
                }

                let csv_data = csv_writer.into_inner().unwrap().into_inner();
                let _ = zip_writer.start_file::<String, ()>(
                    "results.csv".to_string(),
                    FileOptions::default().compression_method(zip::CompressionMethod::Deflated),
                );
                let _ = zip_writer.write_all(&csv_data);

                let _ = zip_writer.finish();

                log::info!("The thing has been done!");

                Some(zip_buffer.into_inner())
            });

            self.zipped_results = Some(promise);
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
                            ui.label("🎉");
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
                            log::info!("Zhu Li! Do the thing!");
                            self.generate_reports();
                            ctx.request_repaint();
                        }
                    }
                });

                columns[4].vertical(|ui| {
                    if let Some(promise) = &self.zipped_results {
                        if let Some(result) = promise.ready() {
                            if let Some(zipfile) = result.clone() {
                                download_button(ui, "💾 Save results as zip file", zipfile);
                            }
                        } else {
                            ui.spinner();
                        }
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
                                    let pdf = pdf::file::FileOptions::cached().load(data).unwrap();
                                    let container = PdfContainer { pdf_file: pdf };

                                    self.container = Some(Arc::new(Mutex::new(container)));
                                }
                                "image/tiff" => {
                                    let buffer = std::io::Cursor::new(data);
                                    let tiff = tiff::decoder::Decoder::new(buffer).unwrap();
                                    let container = TiffContainer { decoder: tiff };
                                    self.container = Some(Arc::new(Mutex::new(container)));
                                }
                                "image/png" => {
                                    let image = image::load_from_memory_with_format(
                                        &data,
                                        image::ImageFormat::Png,
                                    )
                                    .unwrap();
                                    let container = SingleImageContainer { image: image };
                                    self.container = Some(Arc::new(Mutex::new(container)));
                                }
                                "image/jpeg" => {
                                    let image = image::load_from_memory_with_format(
                                        &data,
                                        image::ImageFormat::Jpeg,
                                    )
                                    .unwrap();
                                    let container = SingleImageContainer { image: image };
                                    self.container = Some(Arc::new(Mutex::new(container)));
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
