use crate::image_container::{ImageContainer, PdfContainer, SingleImageContainer, TiffContainer};
use crate::image_helpers::rgb_to_egui_color_image;
use crate::report::ImageReport;
use crate::scan::Scan;
use crate::template::{ExamKey, Template};
use crate::template_scan::TemplateScan;
use crate::webapp::utils::{download_button, upload_button, FileType};
use crate::webapp::webapp::StateView;
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
    pub template: Option<Template>,
    pub key: Option<ExamKey>,
    raw_container_data: Option<Vec<u8>>,
    zipped_results: Rc<RefCell<Option<Vec<u8>>>>,
    data_channel: (Sender<(FileType, Vec<u8>)>, Receiver<(FileType, Vec<u8>)>),
    preview_image: Rc<RefCell<Option<image::RgbImage>>>,
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
            preview_image: self.preview_image.clone(),
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
            preview_image: Rc::new(RefCell::new(None)),
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
                let container = SingleImageContainer::from_data_with_format(
                    &clonedata,
                    image::ImageFormat::Png,
                );
                return Some(Box::new(container));
            }
            "image/jpeg" => {
                let container = SingleImageContainer::from_data_with_format(
                    &clonedata,
                    image::ImageFormat::Jpeg,
                );
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
    fn simplified_update_view(&mut self, ctx: &Context, width: f64, height: f64) {
        egui::CentralPanel::default().show(ctx, |ui| {
            upload_button(
                ui,
                &ctx,
                "📂 Open Image",
                FileType::Container,
                self.data_channel.0.clone(),
            );

            if self.raw_container_data.is_some() && self.preview_texture.is_none() {
                if let Some(mut container) =
                    raw_data_to_container(&self.raw_container_data.clone().unwrap())
                {
                    let img = container.to_iter().next().expect("could not open image");

                    let scan = Scan {
                        image: img,
                    };

                    let template_scan = TemplateScan::new(self.template.as_ref().unwrap(), scan);
                    let report = template_scan.generate_image_report(
                        &self.key.clone().unwrap(),
                        &String::new(),
                    );
                    *self.preview_image.borrow_mut() = Some(report.image.clone());
                    *self.status.borrow_mut() = Some(format!(
                        "{} points (version {}, student ID {})",
                        report.score,
                        report.version.unwrap_or(0) + 1,
                        report.sid.unwrap_or(0)
                    ));
                }
            } else if self.status.borrow().is_none() {
                *self.status.borrow_mut() = Some("Only use pictures that are roughly A4 with the whole visible area being covered by the bubble sheet.".into());
            }

            if let Some(status) = &*self.status.borrow() {
                ui.label(status);
            }
            if let Some(texture) = &self.preview_texture {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add(
                        egui::Image::new(texture)
                            .max_width(2.0 * width as f32)
                            .max_height(3.0 * height as f32),
                    );
                });
            }
        });
    }
    fn full_update_view(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(5, |columns| {
                columns[0].vertical(|ui| {
                    ui.horizontal(|ui| {
                        upload_button(
                            ui,
                            &ctx,
                            "📂 Open Template",
                            FileType::Template,
                            self.data_channel.0.clone(),
                        );
                        if self.template.is_some() {
                            ui.label("🎉");
                        }
                    });
                    ui.label("Open a template file (.json).");
                });
                columns[1].vertical(|ui| {
                    ui.horizontal(|ui| {
                        upload_button(
                            ui,
                            &ctx,
                            "📂 Open Exam Key",
                            FileType::Key,
                            self.data_channel.0.clone(),
                        );
                        if self.key.is_some() {
                            ui.label("👍");
                        }
                    });
                    ui.label("Open an exam key (.json).");
                });
                columns[2].vertical(|ui| {
                    ui.horizontal(|ui| {
                        upload_button(
                            ui,
                            &ctx,
                            "📂 Open Container",
                            FileType::Container,
                            self.data_channel.0.clone(),
                        );
                        if self.raw_container_data.is_some() {
                            ui.label("❤");
                        }
                    });
                    ui.label("Open an image container (.pdf, .tiff, .jpg, .png)");
                });
                columns[3].vertical(|ui| {
                    if self.template.is_some()
                        && self.key.is_some()
                        && self.raw_container_data.is_some()
                    {
                        if ui.button("🚀 Do the thing!").clicked() {
                            log::info!("Zhu Li! Do the thing!");
                            self.preview_texture = None;
                            self.preview_image = Rc::new(RefCell::new(None));
                            self.zipped_results = Rc::new(RefCell::new(None));
                            let mut cloned_self = self.clone();
                            let ctx = ctx.clone();
                            spawn_local(async move {
                                cloned_self.generate_reports(&ctx).await;
                            });
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

            if let Some(texture) = &self.preview_texture {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add(egui::Image::new(texture));
                });
            }
        });
    }

    pub async fn generate_reports(&mut self, ctx: &Context) {
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
                gloo_timers::future::TimeoutFuture::new(1).await;
                for chunk in &iterator.chunks(chunksize) {
                    let images: Vec<image::GrayImage> = chunk.collect();
                    let results: Vec<ImageReport> = images
                        .into_iter()
                        .enumerate()
                        .map(|(idx, img)| {
                            log::info!("processing {}", turn * chunksize + idx);
                            let scan = Scan { image: img };
                            let template_scan = TemplateScan::new(&template, scan);
                            template_scan.generate_image_report(
                                &key,
                                &format!("page{}", idx + turn * chunksize),
                            )
                        })
                        .collect();

                    for r in &results {
                        let _ = r.add_to_zip(&mut zip_writer, &mut csv_writer);
                    }

                    if self.preview_image.borrow().is_none() {
                        *self.preview_image.borrow_mut() = Some(results[0].image.clone());
                    }

                    turn += 1;
                    *self.status.borrow_mut() =
                        Some(format!("processed {} scans", turn * chunksize));
                    ctx.request_repaint();
                    gloo_timers::future::TimeoutFuture::new(1).await;
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

impl StateView for GenerateReport {
    fn get_key(&self) -> Option<&ExamKey> {
        self.key.as_ref()
    }

    fn get_template(&self) -> Option<&Template> {
        self.template.as_ref()
    }

    fn set_key(&mut self, key: Option<ExamKey>) {
        self.key = key;
    }

    fn set_template(&mut self, template: Option<Template>) {
        self.template = template;
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
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
        } // check if we need to create the texture
        if self.preview_image.borrow().is_some() && self.preview_texture.is_none() {
            log::info!("creating texture");
            let display = rgb_to_egui_color_image(&self.preview_image.borrow().clone().unwrap());
            self.preview_texture =
                Some(ctx.load_texture("displayed_image", display, egui::TextureOptions::default()));
        }

        let window = web_sys::window().expect("could not get window");
        let dimensions = window.inner_width().and_then(|x| {
            window.inner_height().and_then(|y| {
                let x_pixels = x.as_f64().ok_or(wasm_bindgen::JsValue::NULL)?;
                let y_pixels = y.as_f64().ok_or(wasm_bindgen::JsValue::NULL)?;
                Ok((x_pixels, y_pixels))
            })
        });

        match dimensions {
            Ok((x, y)) if x < y && self.template.is_some() && self.key.is_some() => {
                self.simplified_update_view(ctx, x, y)
            }
            _ => self.full_update_view(ctx),
        }
    }
}
