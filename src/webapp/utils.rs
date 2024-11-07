use crate::point::Point;
use crate::template::Box;
use crate::template::Question;
use crate::template::Template;
use crate::webapp::create_template::{
    CircleSettings, LayoutSettings, PositionSettings, QuestionSettings,
};
use eframe::egui::{Context, Ui};
use std::future::Future;
use tokio::sync::mpsc::Sender;

pub enum FileType {
    Template,
    Key,
    Container,
    TemplateImage,
}
pub fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

pub fn upload_button(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    label: &str,
    file_type: FileType,
    sender: Sender<(FileType, Vec<u8>)>,
) {
    if ui.button(label).clicked() {
        let sender = sender.clone();
        let ctx = ctx.clone();
        let task = rfd::AsyncFileDialog::new().pick_file();
        execute(async move {
            if let Some(file) = task.await {
                let bytes = file.read().await;
                if let Err(e) = sender.send((file_type, bytes)).await {
                    log::error!("Failed to send file data: {}", e);
                } else {
                    log::info!("File data sent successfully");
                }
                ctx.request_repaint();
            }
        });
    }
}

pub fn download_button(ui: &mut egui::Ui, label: &str, data: Vec<u8>) {
    if ui.button(label).clicked() {
        let task = rfd::AsyncFileDialog::new().save_file();
        execute(async move {
            let file = task.await;
            if let Some(file) = file {
                _ = file.write(&data).await;
            }
        });
    }
}
pub fn template_from_settings(
    qss: &QuestionSettings,
    ls: &LayoutSettings,
    ps: &PositionSettings,
    cs: &CircleSettings,
) -> Template {
    let qs = qss.num_qs;
    let idqs = qss.num_id_qs;
    let h = ls.box_height;
    let w = ls.box_width;
    let pad_h = ls.padding_horizontal;
    let pad_v = ls.padding_vertical;
    let vs = qss.num_versions;
    let answers = qss.num_answers;

    let first_q = ps.first_q;
    let first_vq = ps.first_vq;
    let first_idq = ps.first_id_q;

    Template {
        id_questions: question_builder(first_idq, 10, w, h, pad_h, pad_v, idqs, &"id.".to_string()),
        version: Question {
            id: "version".to_string(),
            boxes: box_builder(first_vq, w, h, pad_h, vs),
        },
        questions: question_builder(first_q, answers, w, h, pad_h, pad_v, qs, &"q.".to_string()),
        circle_centers: cs.centers,
        circle_radius: cs.radius,
        height: ls.height,
        width: ls.width,
    }
}

fn box_builder(a: Point, w: u32, h: u32, pad_h: u32, count: u32) -> Vec<Box> {
    let mut start = a;
    let mut result = Vec::new();
    for i in 0..count {
        let stop = Point {
            x: start.x + w,
            y: start.y + h,
        };
        result.push(Box {
            a: start,
            b: stop,
            value: i,
        });
        start = Point {
            x: start.x + w + pad_h,
            y: start.y,
        };
    }

    result
}

pub fn question_builder(
    a: Point,
    answers: u32,
    w: u32,
    h: u32,
    pad_h: u32,
    pad_v: u32,
    count: u32,
    prefix: &String,
) -> Vec<Question> {
    let mut start = a;
    let mut result = Vec::new();

    for i in 1..(count + 1) {
        let boxes = box_builder(start, w, h, pad_h, answers);
        let q = Question {
            id: prefix.to_string() + &i.to_string(),
            boxes: boxes,
        };
        result.push(q);
        start = Point {
            x: start.x,
            y: start.y + h + pad_v,
        };
    }

    result
}
