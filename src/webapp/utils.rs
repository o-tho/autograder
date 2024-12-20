use crate::point::Point;
use crate::template::Box;
use crate::template::ExamKey;
use crate::template::Question;
use crate::template::Template;
use crate::webapp::create_template::{CircleSettings, LayoutSettings, PositionSettings};
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

pub struct QuestionSettings {
    pub num_qs: u32,
    pub num_id_qs: u32,
    pub num_versions: u32,
    pub num_answers: u32,
}

impl Default for QuestionSettings {
    fn default() -> Self {
        Self {
            num_qs: 20,
            num_id_qs: 9,
            num_versions: 4,
            num_answers: 5,
        }
    }
}

pub fn upload_button(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    label: &str,
    file_type: FileType,
    sender: Sender<(FileType, String, Vec<u8>)>,
) {
    if ui.button(label).clicked() {
        let sender = sender.clone();
        let ctx = ctx.clone();
        let task = rfd::AsyncFileDialog::new().pick_file();
        execute(async move {
            if let Some(file) = task.await {
                let bytes = file.read().await;
                if let Err(e) = sender.send((file_type, file.file_name(), bytes)).await {
                    log::error!("Failed to send file data: {}", e);
                } else {
                    log::info!("File data sent successfully");
                }
                ctx.request_repaint();
            }
        });
    }
}

pub fn download_button(
    ui: &mut egui::Ui,
    label: &str,
    default_save_name: impl Into<String>,
    data: Vec<u8>,
) {
    if ui.button(label).clicked() {
        let task = rfd::AsyncFileDialog::new()
            .set_file_name(default_save_name)
            .save_file();
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
        id_questions: question_builder(first_idq, 10, w, h, pad_h, pad_v, idqs),
        version: Question {
            boxes: box_builder(first_vq, w, h, pad_h, vs),
        },
        questions: question_builder(first_q, answers, w, h, pad_h, pad_v, qs),
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
) -> Vec<Question> {
    let mut start = a;
    let mut result = Vec::new();

    for _i in 1..(count + 1) {
        let boxes = box_builder(start, w, h, pad_h, answers);
        let q = Question { boxes: boxes };
        result.push(q);
        start = Point {
            x: start.x,
            y: start.y + h + pad_v,
        };
    }

    result
}

pub fn encode_key_template(k: &ExamKey, t: &Template) -> String {
    use base64_url::encode;
    use snap::raw::Encoder;
    if let Ok(serialized) = serde_cbor::to_vec(&(k, t)) {
        let mut encoder = Encoder::new();
        if let Ok(compressed) = encoder.compress_vec(&serialized) {
            encode(&compressed)
        } else {
            "".into()
        }
    } else {
        "".into()
    }
}

pub fn decode_key_template(
    encoded: &str,
) -> Result<(ExamKey, Template), std::boxed::Box<dyn std::error::Error>> {
    use base64_url::decode;
    use snap::raw::Decoder;
    let decoded = decode(encoded)?;

    let mut decoder = Decoder::new();
    let decompressed = decoder.decompress_vec(&decoded)?;

    let (k, t): (ExamKey, Template) = serde_cbor::from_slice(&decompressed)?;

    Ok((k, t))
}
