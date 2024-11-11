use crate::image_container::{ImageContainer, PdfContainer, SingleImageContainer, TiffContainer};
use crate::image_helpers::rgb_to_egui_color_image;
use crate::report::ImageReport;
use crate::scan::Scan;
use crate::template::{ExamKey, Template};
use crate::webapp::utils::{download_button, upload_button, FileType};
use egui::Context;
use infer;
use itertools::Itertools;
use serde_json;
use std::cell::RefCell;
use std::io::Cursor;
use std::io::Write;
use std::rc::Rc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use zip::write::FileOptions;
use zip::ZipWriter;

use wasm_bindgen_futures::spawn_local;

pub struct GenerateReport {
    template: Option<Template>,
    key: Option<ExamKey>,
    raw_container_data: Option<Vec<u8>>,
    zipped_results: Rc<RefCell<Option<Vec<u8>>>>,
    data_channel: (Sender<(FileType, Vec<u8>)>, Receiver<(FileType, Vec<u8>)>),
    preview_texture: Option<egui::TextureHandle>,
    status: Rc<RefCell<Option<String>>>,
}

impl Clone for GenerateReport {
    fn clone(&self) -> Self {
        Self {
            template: self.template.clone(),
            key: self.key.clone(),
            raw_container_data: self.raw_container_data.clone(),
            zipped_results: Rc::clone(&self.zipped_results),
            data_channel: channel(50),
            preview_texture: self.preview_texture.clone(),
            status: self.status.clone(),
        }
    }
}

impl Default for GenerateReport {
    fn default() -> Self {
        let (sender, receiver) = channel(50);
        Self {
            template: None,
            key: None,
            raw_container_data: None,
            data_channel: (sender, receiver),
            zipped_results: Rc::new(RefCell::new(None)),
            preview_texture: None,
            status: Rc::new(RefCell::new(None)),
        }
    }
}

fn raw_data_to_container(data: &Vec<u8>) -> Option<Box<dyn ImageContainer + '_>> {
    let clonedata = data.clone();
    if let Some(kind) = infer::get(&clonedata) {
        match kind.mime_type() {
            "application/pdf" => {
                let pdf = pdf::file::FileOptions::cached().load(clonedata).unwrap();
                let container = PdfContainer { pdf_file: pdf };
                return Some(Box::new(container));
            }
            "image/tiff" => {
                let buffer = std::io::Cursor::new(clonedata);
                let tiff = tiff::decoder::Decoder::new(buffer).unwrap();
                let container = TiffContainer { decoder: tiff };
                return Some(Box::new(container));
            }
            "image/png" => {
                let image =
                    image::load_from_memory_with_format(&clonedata, image::ImageFormat::Png)
                        .unwrap();
                let container = SingleImageContainer { image: image };
                return Some(Box::new(container));
            }
            "image/jpeg" => {
                let image =
                    image::load_from_memory_with_format(&clonedata, image::ImageFormat::Jpeg)
                        .unwrap();
                let container = SingleImageContainer { image: image };
                return Some(Box::new(container));
            }
            _ => log::error!(
                "{}",
                format!("Unsupported container format {}", kind.mime_type())
            ),
        }
    }
    None
}

impl GenerateReport {
    pub async fn generate_reports(&mut self) {
        let template = self.template.clone().unwrap();
        let key = self.key.clone().unwrap();

        if let Some(container_data) = self.raw_container_data.clone() {
            let _ = async move {
                let mut zip_buffer = Cursor::new(Vec::new());
                let mut zip_writer = ZipWriter::new(&mut zip_buffer);
                let mut csv_writer = csv::Writer::from_writer(std::io::Cursor::new(Vec::new()));
                let _ = csv_writer.write_record(["Filename", "ID", "Score"]);

                log::info!("Output files are set up, starting to iterate over the input images!");

                let mut container = raw_data_to_container(&container_data).unwrap();
                let iterator = container.to_iter();

                let mut turn = 0;
                let chunksize = 20;

                log::info!(
                    "Working on images {}-{}",
                    turn * chunksize,
                    (turn + 1) * chunksize
                );
                *self.status.borrow_mut() =
                    Some(format!("Working on the first {} scans", chunksize));
                gloo_timers::future::TimeoutFuture::new(50).await;
                for chunk in &iterator.chunks(chunksize) {
                    let images: Vec<image::GrayImage> = chunk.collect();
                    let results: Vec<ImageReport> = images
                        .iter()
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
                    *self.status.borrow_mut() =
                        Some(format!("processed {} scans", turn * chunksize));
                    gloo_timers::future::TimeoutFuture::new(50).await;
                }

                let csv_data = csv_writer.into_inner().unwrap().into_inner();
                let _ = zip_writer.start_file::<String, ()>(
                    "results.csv".to_string(),
                    FileOptions::default().compression_method(zip::CompressionMethod::Deflated),
                );
                let _ = zip_writer.write_all(&csv_data);

                let _ = zip_writer.finish();

                log::info!("The thing has been done!");

                *self.zipped_results.borrow_mut() = Some(zip_buffer.into_inner());
            }
            .await;
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
                        if self.raw_container_data.is_some() {
                            ui.label("❤");
                        }
                    });
                    ui.label("Upload an image container (.pdf, .tiff, .jpg, .png)");
                });
                columns[3].vertical(|ui| {
                    if self.template.is_some()
                        && self.key.is_some()
                        && self.raw_container_data.is_some()
                    {
                        if ui.button("🚀 Do the thing!").clicked() {
                            log::info!("Zhu Li! Do the thing!");
                            let mut cloned_self = self.clone();
                            spawn_local(async move {
                                cloned_self.generate_reports().await;
                            });
                            ctx.request_repaint();
                        }
                    }
                });

                columns[4].vertical(|ui| {
                    if let Some(zipped_data) = &*self.zipped_results.borrow() {
                        download_button(ui, "💾 Save results as zip file", zipped_data.clone());
                        self.status = Rc::new(RefCell::new(None));
                    }
                    if let Some(status) = &*self.status.borrow() {
                        ui.label(status);
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
                        log::info!("uploaded data");
                        self.raw_container_data = Some(data);
                    }
                    _ => {}
                }
            }
        });
    }
}
