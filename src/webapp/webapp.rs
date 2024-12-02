use eframe::egui;
use eframe::egui::Context;

use crate::webapp::create_form::CreateForm;
use crate::webapp::create_key::CreateKey;
use crate::webapp::create_magic_link::CreateMagicLink;
use crate::webapp::create_template::CreateTemplate;

use crate::webapp::utils::decode_key_template;

use crate::webapp::generate_report::GenerateReport;
use crate::webapp::help::Help;

pub struct WebApp {
    current_view: ViewType,
    generate_report: GenerateReport,
    create_template: CreateTemplate,
    create_form: CreateForm,
    create_key: CreateKey,
    create_magic_link: CreateMagicLink,
    help: Help,
}

enum ViewType {
    GenerateReport,
    CreateTemplate,
    CreateForm,
    CreateKey,
    CreateMagicLink,
    Help,
}

impl Default for WebApp {
    fn default() -> Self {
        Self {
            current_view: ViewType::Help,
            generate_report: GenerateReport::default(),
            create_form: CreateForm::default(),
            create_magic_link: CreateMagicLink::default(),
            create_template: CreateTemplate::default(),
            create_key: CreateKey::default(),
            help: Help::default(),
        }
    }
}

impl eframe::App for WebApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        let location = web_sys::window().expect("huge websys error").location();

        if !location.hash().unwrap().is_empty() && self.generate_report.template.is_none() {
            let result = decode_key_template(&location.hash().unwrap().as_str()[1..]);
            if let Ok((key, template)) = result {
                self.current_view = ViewType::GenerateReport;
                self.generate_report.template = Some(template);
                self.generate_report.key = Some(key);
            }
        }
        // Navigation bar
        egui::TopBottomPanel::top("nav_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Generate Report").clicked() {
                    self.current_view = ViewType::GenerateReport;
                }
                if ui.button("Create Form").clicked() {
                    self.current_view = ViewType::CreateForm;
                    let _ = location.set_hash("");
                }
                if ui.button("Create Template").clicked() {
                    self.current_view = ViewType::CreateTemplate;
                    let _ = location.set_hash("");
                }
                if ui.button("Create Key").clicked() {
                    self.current_view = ViewType::CreateKey;
                    let _ = location.set_hash("");
                }
                if ui.button("Create Magic Link").clicked() {
                    self.current_view = ViewType::CreateMagicLink;
                    let _ = location.set_hash("");
                }
                if ui.button("Help").clicked() {
                    self.current_view = ViewType::Help;
                    let _ = location.set_hash("");
                }
            });
        });

        match self.current_view {
            ViewType::GenerateReport => self.generate_report.update(ctx, frame),
            ViewType::CreateForm => self.create_form.update(ctx, frame),
            ViewType::CreateTemplate => self.create_template.update(ctx, frame),
            ViewType::CreateKey => self.create_key.update(ctx, frame),
            ViewType::CreateMagicLink => self.create_magic_link.update(ctx, frame),
            ViewType::Help => self.help.update(ctx, frame),
        }
    }
}
