use eframe::egui::{self, Context, ScrollArea};
use eframe::Frame;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use ehttp::Request;
use poll_promise::Promise;
use regex::Regex;
use wasm_bindgen::prelude::*;

const README_CONTENT: &str = include_str!("../../README.md");

// Function to get the origin (base URL) of the current page
pub struct Help {}

impl Default for Help {
    fn default() -> Self {
        Self {}
    }
}

fn prepare_markdown_content(content: &str) -> String {
    let origin = web_sys::window()
        .expect("no global `window` exists")
        .location()
        .origin()
        .expect("failed to get location origin");

    let modified = content.replace("](assets/", &format!("]({}/assets/", origin));
    modified
}

impl Help {
    pub fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                let mut cache = CommonMarkCache::default();
                // Modify image paths to use absolute URLs with the protocol
                let modified_content = prepare_markdown_content(README_CONTENT);
                CommonMarkViewer::new().show(ui, &mut cache, &modified_content);
            });
        });
    }
}
