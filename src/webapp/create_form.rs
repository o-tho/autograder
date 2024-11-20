use crate::image_helpers::binary_image_from_image;
use crate::scan::Scan;
use crate::template::Template;
use crate::typst_helpers::{typst_frame_to_template, TypstWrapper};
use crate::webapp::utils::{download_button, QuestionSettings};
use eframe::egui::{Context, ScrollArea};
use eframe::Frame;

pub struct CreateForm {
    pub question_settings: QuestionSettings,
    pub preview: Option<egui::TextureHandle>,
    pub pdf: Option<Vec<u8>>,
    pub template: Option<Template>,
    pub png: Option<Vec<u8>>,
}

impl Default for CreateForm {
    fn default() -> Self {
        Self {
            question_settings: QuestionSettings::default(),
            preview: None,
            pdf: None,
            template: None,
            png: None,
        }
    }
}

impl CreateForm {
    pub fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        eframe::egui::SidePanel::left("settings_panel")
            .resizable(false)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Settings");
                ui.add_space(10.0);

                ui.add(
                    egui::Slider::new(&mut self.question_settings.num_qs, 1..=50)
                        .text("Number of Questions"),
                );

                ui.add(
                    egui::Slider::new(&mut self.question_settings.num_id_qs, 1..=15)
                        .text("ID Questions"),
                );

                ui.add(
                    egui::Slider::new(&mut self.question_settings.num_versions, 1..=8)
                        .text("Versions"),
                );

                ui.add(
                    egui::Slider::new(&mut self.question_settings.num_answers, 2..=8)
                        .text("Answers per Question"),
                );

                ui.add_space(20.0);

                if ui.button("Generate").clicked() {
                    let tmpl = include_str!("../../assets/formtemplate.typ");

                    let code = format!(
                        r#"
#let num_qs = {}
#let num_idqs = {}
#let num_answers = {}
#let num_versions = {}
{}
"#,
                        self.question_settings.num_qs,
                        self.question_settings.num_id_qs,
                        self.question_settings.num_answers,
                        self.question_settings.num_versions,
                        tmpl
                    );
                    let wrapper = TypstWrapper::new(code);

                    let document = typst::compile(&wrapper)
                        .output
                        .expect("Error from Typst. This really should not happen. So sorry.");

                    let scale = 3.0;
                    let template = typst_frame_to_template(&document.pages[0].frame, scale);
                    self.template = Some(template.clone());
                    let pdf =
                        typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).expect("bla");

                    self.pdf = Some(pdf.clone());

                    let png = typst_render::render(&document.pages[0], scale as f32)
                        .encode_png()
                        .unwrap();

                    self.png = Some(png.clone());

                    let dynimage = image::load_from_memory(&png).unwrap();
                    let scan = Scan {
                        img: binary_image_from_image(dynimage),
                        transformation: None,
                    };
                    let circled = scan.circle_everything(&template);
                    let dynamic_image = image::DynamicImage::ImageRgb8(circled);

                    let size = [dynamic_image.width() as _, dynamic_image.height() as _];
                    let image_buffer = dynamic_image.to_rgba8();
                    let pixels = image_buffer.as_flat_samples();

                    let texture = ctx.load_texture(
                        "preview_image",
                        egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
                        egui::TextureOptions::default(),
                    );
                    self.preview = Some(texture);

                    download_button(
                        ui,
                        "💾 Save Template as json",
                        serde_json::to_vec(&template).unwrap(),
                    );
                    download_button(ui, "💾 Save form as PNG", png);
                }

                if let Some(template) = &self.template {
                    download_button(
                        ui,
                        "💾 Save template as JSON",
                        serde_json::to_vec(&template).unwrap(),
                    );
                }
                if let Some(pdf) = &self.pdf {
                    download_button(ui, "💾 Save form as PDF", pdf.to_vec());
                }
                if let Some(png) = &self.png {
                    download_button(ui, "💾 Save form as PNG", png.to_vec());
                }
            });

        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Preview");
            if let Some(texture) = &self.preview {
                ScrollArea::both().show(ui, |ui| {
                    // Display the rendered image or a placeholder if no image is available
                    ui.add(egui::Image::new(texture));
                });
            } else {
                ui.label("No preview available");
            }
        });
    }
}
