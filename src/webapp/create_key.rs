use crate::template::{CorrectAnswer, ExamKey};
use crate::webapp::utils::download_button;
use crate::webapp::webapp::StateView;
use eframe::egui::{Context, Slider, TextEdit};
use eframe::Frame;

pub struct CreateKey {
    number_of_versions: usize,
    inputs: Vec<String>,
    key: ExamKey,
}

impl Default for CreateKey {
    fn default() -> Self {
        Self {
            number_of_versions: 4,
            inputs: vec![String::new(); 4], // Start with 4 empty strings
            key: Vec::new(),
        }
    }
}

fn convert_to_vector(input: &str) -> Vec<CorrectAnswer> {
    let mut result = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        let c = c.to_ascii_uppercase();
        if c == '(' {
            // Collect multiple options within parentheses
            let mut options = Vec::new();
            while let Some(&next) = chars.peek() {
                let next = next.to_ascii_uppercase();
                if next == ')' {
                    chars.next(); // consume the closing parenthesis
                    break;
                }
                if ('A'..='Z').contains(&next) {
                    options.push((next as u32 - 'A' as u32) as u32);
                }
                chars.next();
            }
            result.push(CorrectAnswer::OneOf(options));
        } else if ('A'..='Z').contains(&c) {
            // Single letter is Exactly
            result.push(CorrectAnswer::Exactly(c as u32 - 'A' as u32));
        }
    }

    result
}

fn convert_to_string(input: &Vec<CorrectAnswer>) -> String {
    input
        .iter()
        .map(|key| match key {
            CorrectAnswer::Exactly(n) => char::from(b'A' + (*n as u8)).to_string(),
            CorrectAnswer::OneOf(options) => {
                let option_chars: String = options
                    .iter()
                    .map(|&n| char::from(b'A' + (n as u8)))
                    .collect();
                format!("({})", option_chars)
            }
        })
        .collect()
}

impl StateView for CreateKey {
    fn get_key(&self) -> Option<&ExamKey> {
        if self.key.len() > 0 {
            Some(self.key.as_ref())
        } else {
            None
        }
    }

    fn set_key(&mut self, key: Option<ExamKey>) {
        if let Some(vec) = key {
            self.number_of_versions = vec.len();
            self.inputs = vec.iter().map(&convert_to_string).collect();
            self.key = vec;
        } else {
            self.key = vec![];
        }
    }
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Create Key");

            ui.horizontal(|ui| {
                ui.label("Number of Versions:");
                ui.add(Slider::new(&mut self.number_of_versions, 1..=10).text("versions"));
            });

            if self.inputs.len() != self.number_of_versions {
                self.inputs.resize(self.number_of_versions, String::new());
            }

            for i in 0..self.number_of_versions {
                ui.horizontal(|ui| {
                    ui.label(format!("Version {}:", i + 1));
                    ui.add(
                        TextEdit::singleline(&mut self.inputs[i])
                            .hint_text("Enter sequence like ABCDE"),
                    );
                    let answer_count = convert_to_vector(&self.inputs[i]).len();
                    ui.label(format!("({} answers)", answer_count));
                });
            }
            self.key = self
                .inputs
                .iter()
                .map(|input| convert_to_vector(input))
                .collect();

            ui.label("If you enter a string like AB(CD)E, it means that in the third question both C and D would be graded as correct, whereas for the other questions only a single choice is counted as correct.");

            download_button(
                ui,
                "💾 Save Key as json",
                serde_json::to_vec(&self.key).unwrap(),
            );
        });
    }
}
