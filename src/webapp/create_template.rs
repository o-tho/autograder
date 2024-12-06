use crate::image_helpers::{binary_image_from_image, rgb_to_egui_color_image};
use crate::point::Point;
use crate::scan::Scan;
use crate::template::Template;
use crate::webapp::utils::{
    download_button, template_from_settings, upload_button, FileType, QuestionSettings,
};
use crate::webapp::webapp::StateView;
use eframe::egui::{CentralPanel, Context, ScrollArea, SidePanel, TextEdit, Ui};
use eframe::Frame;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct CreateTemplate {
    question_settings: QuestionSettings,
    layout_settings: LayoutSettings,
    position_settings: PositionSettings,
    circle_settings: CircleSettings,
    preview_texture: Option<egui::TextureHandle>,
    original_image: Option<image::DynamicImage>,
    template: Option<Template>,
    data_channel: (Sender<(FileType, Vec<u8>)>, Receiver<(FileType, Vec<u8>)>),
}

impl Default for CreateTemplate {
    fn default() -> Self {
        let (sender, receiver) = channel(50);
        Self {
            question_settings: QuestionSettings::default(),
            layout_settings: LayoutSettings::default(),
            position_settings: PositionSettings::default(),
            circle_settings: CircleSettings::default(),
            preview_texture: None,
            original_image: None,
            template: None,
            data_channel: (sender, receiver),
        }
    }
}

fn text_box_with_label(ui: &mut Ui, label: &str, value: &mut u32) {
    ui.horizontal(|ui| {
        ui.add_sized([180.0, 0.0], egui::Label::new(label)); // Fixed width label
        let mut text = value.to_string(); // Convert `u32` to `String`

        // Create the text edit field
        ui.add(TextEdit::singleline(&mut text).desired_width(40.0));

        // Update the `u32` value if the text can be parsed successfully
        if let Ok(parsed_value) = text.parse() {
            *value = parsed_value;
        }
    });
}

fn text_box_pair_with_label(ui: &mut Ui, label: &str, x_value: &mut u32, y_value: &mut u32) {
    ui.horizontal(|ui| {
        ui.add_sized([180.0, 0.0], egui::Label::new(label)); // Fixed width label

        // Text box for the `x` value
        let mut x_text = x_value.to_string();
        ui.add(TextEdit::singleline(&mut x_text).desired_width(40.0));
        if let Ok(parsed_x) = x_text.parse() {
            *x_value = parsed_x;
        }

        // Text box for the `y` value
        let mut y_text = y_value.to_string();
        ui.add(TextEdit::singleline(&mut y_text).desired_width(40.0));
        if let Ok(parsed_y) = y_text.parse() {
            *y_value = parsed_y;
        }
    });
}

impl StateView for CreateTemplate {
    fn get_template(&self) -> Option<&Template> {
        self.template.as_ref()
    }

    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        SidePanel::left("controls_panel")
            .resizable(true)
            .min_width(ctx.screen_rect().width() * 0.25) // Set to 30% of the screen width
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Template Settings");

                    ui.group(|ui| {
                        ui.label("Open the template image (NOT pdf)");
                        upload_button(
                            ui,
                            &ctx,
                            "📂 Open image of template",
                            FileType::TemplateImage,
                            self.data_channel.0.clone(),
                        );
                    });

                    // Question Settings Section
                    ui.group(|ui| {
                        ui.label("Question Settings");
                        text_box_with_label(
                            ui,
                            "Number of Questions:",
                            &mut self.question_settings.num_qs,
                        );
                        text_box_with_label(
                            ui,
                            "Number of ID Questions:",
                            &mut self.question_settings.num_id_qs,
                        );
                        text_box_with_label(
                            ui,
                            "Number of Versions:",
                            &mut self.question_settings.num_versions,
                        );
                        text_box_with_label(
                            ui,
                            "Number of Answers:",
                            &mut self.question_settings.num_answers,
                        );
                    });

                    // Layout Settings Section
                    ui.group(|ui| {
                        ui.label("Layout Settings");
                        text_box_with_label(
                            ui,
                            "Box Height:",
                            &mut self.layout_settings.box_height,
                        );
                        text_box_with_label(ui, "Box Width:", &mut self.layout_settings.box_width);
                        text_box_with_label(
                            ui,
                            "Horizontal Padding:",
                            &mut self.layout_settings.padding_horizontal,
                        );
                        text_box_with_label(
                            ui,
                            "Vertical Padding:",
                            &mut self.layout_settings.padding_vertical,
                        );
                    });

                    // Position Settings Section
                    ui.group(|ui| {
                        ui.label("Position Settings");
                        text_box_pair_with_label(
                            ui,
                            "First Question (x, y):",
                            &mut self.position_settings.first_q.x,
                            &mut self.position_settings.first_q.y,
                        );
                        text_box_pair_with_label(
                            ui,
                            "First Version Question (x, y):",
                            &mut self.position_settings.first_vq.x,
                            &mut self.position_settings.first_vq.y,
                        );
                        text_box_pair_with_label(
                            ui,
                            "First ID Question (x, y):",
                            &mut self.position_settings.first_id_q.x,
                            &mut self.position_settings.first_id_q.y,
                        );
                    });

                    // Circle Settings Section
                    ui.group(|ui| {
                        ui.label("Circle Settings");
                        for (i, center) in self.circle_settings.centers.iter_mut().enumerate() {
                            text_box_pair_with_label(
                                ui,
                                &format!("Circle {} Position (x, y):", i + 1),
                                &mut center.x,
                                &mut center.y,
                            );
                        }
                        text_box_with_label(
                            ui,
                            "(Inner) circle radius",
                            &mut self.circle_settings.radius,
                        );
                        if self.original_image.is_some() {
                            if ui
                                .button("🎯 Find precise circle center & radius")
                                .clicked()
                            {
                                let scan = Scan {
                                    img: binary_image_from_image(
                                        self.original_image.clone().unwrap(),
                                    ),
                                    transformation: None,
                                };

                                if let Some(circle_centers_with_radius) = scan
                                    .real_centers_with_radius(
                                        self.circle_settings.centers,
                                        self.circle_settings.radius,
                                    )
                                {
                                    self.circle_settings.centers = circle_centers_with_radius.0;
                                    self.circle_settings.radius = circle_centers_with_radius.1;
                                }
                            };
                        }
                    });
                    if self.original_image.is_some() {
                        if ui.button("📄 Preview!").clicked() {
                            let template = self.to_template();
                            let scan = Scan {
                                img: binary_image_from_image(self.original_image.clone().unwrap()),
                                transformation: None,
                            };

                            let result = scan.circle_everything(&template);
                            let dynamic_image = image::DynamicImage::ImageRgb8(result);
                            self.template = Some(template);
                            self.update_texture(ui, &dynamic_image);
                        }
                        download_button(
                            ui,
                            "💾 Download Json",
                            serde_json::to_vec(&self.to_template()).unwrap(),
                        );
                    }
                });
            });
        // Right Panel: Preview (remaining 70% of the width)
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Preview");

            ScrollArea::both().show(ui, |ui| {
                // Display the rendered image or a placeholder if no image is available
                if let Some(texture) = &self.preview_texture {
                    ui.add(egui::Image::new(texture));
                } else {
                    ui.label("No preview available.");
                }
            });

            while let Ok((file_type, data)) = self.data_channel.1.try_recv() {
                match file_type {
                    FileType::TemplateImage => {
                        if let Ok(image) = image::load_from_memory(&data) {
                            self.original_image = Some(image.clone());
                            self.update_texture(ui, &image);
                            log::info!("loaded image");
                        } else {
                            log::error!("could not read image");
                        }
                    }

                    _ => {}
                }
            }
        });
    }
}
impl CreateTemplate {
    fn update_texture(&mut self, ui: &mut Ui, image: &image::DynamicImage) {
        let rgb = image.clone().into_rgb8();
        let eguicolor = rgb_to_egui_color_image(&rgb);
        self.preview_texture = Some(ui.ctx().load_texture(
            "displayed_image",
            eguicolor,
            egui::TextureOptions::default(),
        ));
    }
    pub fn to_template(&self) -> Template {
        template_from_settings(
            &self.question_settings,
            &self.layout_settings,
            &self.position_settings,
            &self.circle_settings,
        )
    }
}
pub struct LayoutSettings {
    pub box_height: u32,
    pub box_width: u32,
    pub padding_horizontal: u32,
    pub padding_vertical: u32,
    pub height: u32,
    pub width: u32,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            box_height: 41,
            box_width: 46,
            padding_horizontal: 15,
            padding_vertical: 7,
            height: 3487,
            width: 2468,
        }
    }
}

pub struct PositionSettings {
    pub first_q: Point,
    pub first_vq: Point,
    pub first_id_q: Point,
}

impl Default for PositionSettings {
    fn default() -> Self {
        Self {
            first_q: Point { x: 481, y: 400 },
            first_vq: Point { x: 1402, y: 784 },
            first_id_q: Point { x: 1402, y: 928 },
        }
    }
}

pub struct CircleSettings {
    pub centers: [Point; 3],
    pub radius: u32,
}

impl Default for CircleSettings {
    fn default() -> Self {
        Self {
            centers: [
                Point { x: 294, y: 268 },
                Point { x: 2242, y: 268 },
                Point { x: 2242, y: 3114 },
            ],
            radius: 45,
        }
    }
}
