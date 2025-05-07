use eframe::egui;
use eframe::egui::Context;

use crate::webapp::create_form::CreateForm;
use crate::webapp::create_key::CreateKey;
use crate::webapp::create_magic_link::CreateMagicLink;

use crate::webapp::utils::decode_key_template;

use crate::webapp::generate_report::GenerateReport;
use crate::webapp::help::Help;

use crate::template::{ExamKey, Template};

pub trait StateView {
    fn get_key(&self) -> Option<&ExamKey> {
        None
    }

    fn get_template(&self) -> Option<&Template> {
        None
    }

    fn set_key(&mut self, _key: Option<ExamKey>) {}
    fn set_template(&mut self, _template: Option<Template>) {}
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame);
}

pub enum View {
    GenerateReport(GenerateReport),
    CreateForm(CreateForm),
    CreateKey(CreateKey),
    CreateMagicLink(CreateMagicLink),
    Help(Help),
}

impl View {
    fn as_state_view(&self) -> Option<&dyn StateView> {
        match self {
            View::GenerateReport(v) => Some(v),
            View::CreateForm(v) => Some(v),
            View::CreateKey(v) => Some(v),
            View::CreateMagicLink(v) => Some(v),
            _ => None,
        }
    }

    fn as_state_view_mut(&mut self) -> Option<&mut dyn StateView> {
        match self {
            View::GenerateReport(v) => Some(v),
            View::CreateForm(v) => Some(v),
            View::CreateKey(v) => Some(v),
            View::CreateMagicLink(v) => Some(v),
            _ => None,
        }
    }

    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        match self {
            View::GenerateReport(v) => v.update(ctx, frame),
            View::CreateForm(v) => v.update(ctx, frame),
            View::CreateKey(v) => v.update(ctx, frame),
            View::CreateMagicLink(v) => v.update(ctx, frame),
            View::Help(v) => v.update(ctx, frame),
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct SharedState {
    key: Option<ExamKey>,
    template: Option<Template>,
}
pub struct WebApp {
    shared_state: SharedState,
    current_view: View,
}

impl SharedState {
    fn sync_from_view(&mut self, view: &dyn StateView) {
        if let Some(key) = view.get_key() {
            self.key = Some(key.clone());
        }
        if let Some(template) = view.get_template() {
            self.template = Some(template.clone());
        }
    }

    fn sync_to_view(&self, view: &mut dyn StateView) {
        if self.key.is_some() {
            view.set_key(self.key.clone());
        }
        if self.template.is_some() {
            view.set_template(self.template.clone());
        }
    }
}

impl Default for WebApp {
    fn default() -> Self {
        Self {
            current_view: View::Help(Help::default()),
            shared_state: SharedState {
                key: None,
                template: None,
            },
        }
    }
}

impl eframe::App for WebApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        let location = web_sys::window().expect("huge websys error").location();

        // Handle URL hash first
        if !location.hash().unwrap().is_empty() && self.shared_state.template.is_none() {
            let result = decode_key_template(&location.hash().unwrap().as_str()[1..]);
            if let Ok((key, template)) = result {
                self.shared_state.template = Some(template);
                self.shared_state.key = Some(key);
                self.current_view = View::GenerateReport(GenerateReport::default());
                if let Some(view) = self.current_view.as_state_view_mut() {
                    self.shared_state.sync_to_view(view);
                }
            }
        }

        // Navigation bar
        egui::TopBottomPanel::top("nav_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut new_view = None;

                // Collect navigation actions
                if ui.button("Generate Report").clicked() {
                    new_view = Some(View::GenerateReport(GenerateReport::default()));
                }
                if ui.button("Create Form").clicked() {
                    new_view = Some(View::CreateForm(CreateForm::default()));
                    let _ = location.set_hash("");
                }
                if ui.button("Create Key").clicked() {
                    new_view = Some(View::CreateKey(CreateKey::default()));
                    let _ = location.set_hash("");
                }
                if ui.button("Create Magic Link").clicked() {
                    new_view = Some(View::CreateMagicLink(CreateMagicLink::default()));
                    let _ = location.set_hash("");
                }
                if ui.button("Help").clicked() {
                    new_view = Some(View::Help(Help::default()));
                    let _ = location.set_hash("");
                }

                // Handle view transition
                if let Some(new_view) = new_view {
                    // First sync from current view to shared state
                    if let Some(current_view) = self.current_view.as_state_view() {
                        self.shared_state.sync_from_view(current_view);
                    }

                    // Then switch to new view
                    self.current_view = new_view;

                    // Finally sync shared state to new view
                    if let Some(view) = self.current_view.as_state_view_mut() {
                        self.shared_state.sync_to_view(view);
                    }
                }
            });
        });

        // Update the current view
        self.current_view.update(ctx, frame);
    }
}
