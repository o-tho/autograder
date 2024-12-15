use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::Frame;
use typst::layout::FrameItem;
use typst::layout::GroupItem;
use typst::layout::Point as TypstPoint;
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::Library;

use crate::point::Point;
use crate::template::{Box, Question, Template};

// hardcoding these for easy wasm support
pub const BIOLINUM: &[u8] = include_bytes!("../assets/linux-biolinum.regular.ttf");
pub const BIOLINUM_BOLD: &[u8] = include_bytes!("../assets/linux-biolinum.bold.ttf");

#[derive(Debug)]
enum BubbleType {
    Mcq { question: i32, option: String },
    Version { option: String },
    ID { question: i32, digit: i32 },
}

#[derive(Debug)]
struct BubbleInfo {
    top_left_x: f64,
    top_left_y: f64,
    bottom_right_x: f64,
    bottom_right_y: f64,
    bubble_type: BubbleType,
}

pub fn typst_template(num_qs: u32, num_id_qs: u32, num_versions: u32, num_answers: u32) -> String {
    let tmpl = include_str!("../assets/formtemplate.typ");
    format!(
        r#"
#let num_qs = {}
#let num_idqs = {}
#let num_answers = {}
#let num_versions = {}
{}
"#,
        num_qs, num_id_qs, num_answers, num_versions, tmpl
    )
}

pub fn generate_form_and_template(
    num_qs: u32,
    num_id_qs: u32,
    num_versions: u32,
    num_answers: u32,
    scale: f64,
) -> (typst::model::Document, Template) {
    let code = typst_template(num_qs, num_id_qs, num_versions, num_answers);
    let wrapper = TypstWrapper::new(code);
    let document = typst::compile(&wrapper).output.unwrap();
    let frame = &document.pages[0].frame;
    let template = typst_frame_to_template(&frame, scale);

    (document, template)
}

fn extract_bubbles(frame: &Frame) -> Vec<BubbleInfo> {
    use FrameItem;

    let mut bubbles = Vec::new();

    // Skip the first 6 shapes (annuli)
    let mut shape_count = 0;

    for (pos, item) in frame.items() {
        match item {
            FrameItem::Shape(_, _) => {
                shape_count += 1;
                if shape_count <= 6 {
                    continue;
                }
            }
            FrameItem::Group(group) => {
                process_group(group, pos, &mut bubbles);
            }
            _ => {}
        }
    }

    bubbles
}

fn process_group(group: &GroupItem, parent_pos: &TypstPoint, bubbles: &mut Vec<BubbleInfo>) {
    use typst::foundations::Value;
    use typst::introspection::Tag;
    let mut current_metadata = None;
    let mut current_position = None;
    let mut current_size = None;

    for (pos, item) in group.frame.items() {
        let absolute_pos = TypstPoint {
            x: parent_pos.x + pos.x,
            y: parent_pos.y + pos.y,
        };
        match item {
            FrameItem::Tag(Tag::Start(content)) => {
                if let Ok(Value::Dict(dict)) = content.field_by_name("value") {
                    if let Ok(Value::Str(id)) = dict.at("id".into(), None) {
                        current_metadata = Some(id.to_string());
                    }
                }
            }
            FrameItem::Shape(shape, _) => {
                let size = shape.geometry.bbox_size();
                current_position = Some(absolute_pos);
                current_size = Some(size);
            }
            FrameItem::Text(_text_item) => {
                if let (Some(pos), Some(size), Some(id)) = (
                    current_position.take(),
                    current_size.take(),
                    current_metadata.take(),
                ) {
                    if let Some(bubble_type) = parse_bubble_type(&id) {
                        bubbles.push(BubbleInfo {
                            top_left_x: pos.x.to_pt(),
                            top_left_y: pos.y.to_pt(),
                            bottom_right_x: (pos.x + size.x).to_pt(),
                            bottom_right_y: (pos.y + size.y).to_pt(),
                            bubble_type,
                        });
                    }
                }
            }
            FrameItem::Group(nested_group) => {
                process_group(nested_group, &absolute_pos, bubbles);
            }
            _ => {}
        }
    }
}

fn parse_bubble_type(id: &str) -> Option<BubbleType> {
    let parts: Vec<&str> = id.split('-').collect();
    match *(parts.first()?) {
        "mcq" => Some(BubbleType::Mcq {
            question: parts[1].parse().ok()?,
            option: parts[2].to_string(),
        }),
        "version" => Some(BubbleType::Version {
            option: parts[1].to_string(),
        }),
        "id" => Some(BubbleType::ID {
            question: parts[1].parse().ok()?,
            digit: parts[2].parse().ok()?,
        }),
        _ => None,
    }
}

pub fn typst_frame_to_template(frame: &typst::layout::Frame, scale: f64) -> Template {
    use std::collections::HashMap;
    let page_width = 595;
    let page_height = 842;

    // Process annuli from the shapes in the outer frame
    let mut circle_centers = [Point { x: 0, y: 0 }; 3];
    let mut center_count = 0;
    let mut circle_radius = 0;

    let bubbles = extract_bubbles(frame);
    for (pos, item) in frame.items() {
        if let FrameItem::Shape(shape, _) = item {
            if shape.fill
                == Some(typst::visualize::Paint::Solid(
                    typst::visualize::Color::WHITE,
                ))
            {
                let size = shape.geometry.bbox_size();
                let radius = ((size.x / 2.0).to_pt() * scale).round() as u32;
                let center_x = ((pos.x + size.x / 2.0).to_pt() * scale).round() as u32;
                let center_y = ((pos.y + size.y / 2.0).to_pt() * scale).round() as u32;

                circle_centers[center_count] = Point {
                    x: center_x,
                    y: center_y,
                };
                center_count += 1;
                circle_radius = radius;
            }
        }
    }
    let mut mcq_questions: HashMap<i32, Question> = HashMap::new();
    let mut id_questions: HashMap<i32, Question> = HashMap::new();
    let mut version = Question { boxes: Vec::new() };

    for bubble in bubbles {
        let box_data = Box {
            a: Point {
                x: (bubble.top_left_x * scale).round() as u32,
                y: (bubble.top_left_y * scale).round() as u32,
            },
            b: Point {
                x: (bubble.bottom_right_x * scale).round() as u32,
                y: (bubble.bottom_right_y * scale).round() as u32,
            },
            value: match &bubble.bubble_type {
                BubbleType::Mcq { option, .. } => {
                    option.chars().next().unwrap() as u32 - 'A' as u32
                }
                BubbleType::Version { option } => {
                    option.chars().next().unwrap() as u32 - 'A' as u32
                }
                BubbleType::ID { digit, .. } => *digit as u32,
            },
        };

        match bubble.bubble_type {
            BubbleType::Mcq { question, .. } => {
                mcq_questions
                    .entry(question)
                    .or_insert_with(|| Question { boxes: Vec::new() })
                    .boxes
                    .push(box_data);
            }
            BubbleType::Version { .. } => {
                version.boxes.push(box_data);
            }
            BubbleType::ID { question, .. } => {
                id_questions
                    .entry(question)
                    .or_insert_with(|| Question { boxes: Vec::new() })
                    .boxes
                    .push(box_data);
            }
        }
    }

    let mut mcq_questions: Vec<(i32, Question)> = mcq_questions.into_iter().collect();
    mcq_questions.sort_by_key(|(key, _)| *key);
    let mcq_questions: Vec<Question> = mcq_questions.into_iter().map(|(_, q)| q).collect();
    let mut id_questions: Vec<(i32, Question)> = id_questions.into_iter().collect();
    id_questions.sort_by_key(|(key, _)| *key);
    let id_questions: Vec<Question> = id_questions.into_iter().map(|(_, q)| q).collect();

    Template {
        id_questions,
        version,
        questions: mcq_questions,
        circle_centers,
        circle_radius,
        height: (page_height as f64 * scale).round() as u32,
        width: (page_width as f64 * scale).round() as u32,
    }
}

pub struct TypstWrapper {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    document: String,
}

impl TypstWrapper {
    pub fn new(doc: String) -> Self {
        let fonts: Vec<Font> = [BIOLINUM, BIOLINUM_BOLD]
            .into_iter()
            .flat_map(|entry| {
                let buffer = Bytes::from(entry);
                let face_count = ttf_parser::fonts_in_collection(&buffer).unwrap_or(1);
                (0..face_count).map(move |face| {
                    Font::new(buffer.clone(), face).unwrap_or_else(|| panic!("failed to load font"))
                })
            })
            .collect();

        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(FontBook::from_fonts(&fonts)),
            fonts,
            document: doc,
        }
    }
}

impl typst::World for TypstWrapper {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        Source::detached(self.document.clone()).id()
    }

    fn source(&self, _id: FileId) -> FileResult<Source> {
        Ok(Source::detached(self.document.clone()))
    }

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        Err(FileError::Other(Some(
            "This should never have been called".into(),
        )))
    }

    fn font(&self, id: usize) -> Option<Font> {
        self.fonts.get(id).cloned()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let offset = offset.unwrap_or(0);
        let offset = time::UtcOffset::from_hms(offset.try_into().ok()?, 0, 0).ok()?;
        let time = time::OffsetDateTime::now_utc().checked_to_offset(offset)?;
        Some(Datetime::Date(time.date()))
    }
}
