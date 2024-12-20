use crate::template::ExamKey;
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

fn convert_to_vector(input: &str) -> Vec<u32> {
    input
        .chars()
        .filter_map(|c| {
            let c = c.to_ascii_uppercase(); // Convert to uppercase to handle both cases
            if ('A'..='Z').contains(&c) {
                Some((c as u32 - 'A' as u32) as u32) // Map 'A' to 0, 'B' to 1, ..., 'Z' to 25
            } else {
                None // Ignore any non-alphabet characters
            }
        })
        .collect()
}

fn convert_to_string(input: &Vec<u32>) -> String {
    input
        .iter()
        .map(|&num| char::from(b'A' + (num as u8)))
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

            // Slider for the number of versions
            ui.horizontal(|ui| {
                ui.label("Number of Versions:");
                ui.add(Slider::new(&mut self.number_of_versions, 1..=10).text("versions"));
            });

            // Adjust the number of input fields based on the number of versions
            if self.inputs.len() != self.number_of_versions {
                self.inputs.resize(self.number_of_versions, String::new());
            }

            // Display the input text fields
            for i in 0..self.number_of_versions {
                ui.horizontal(|ui| {
                    ui.label(format!("Version {}:", i + 1));
                    ui.add(
                        TextEdit::singleline(&mut self.inputs[i])
                            .hint_text("Enter sequence like ABCDE"),
                    );
                    ui.label(format!("({} answers)", self.inputs[i].len()));
                });
            }
            self.key = self
                .inputs
                .iter()
                .map(|input| convert_to_vector(input))
                .collect();

            // Convert the inputs into a Vec<Vec<u32>> when the user is ready
            download_button(
                ui,
                "💾 Save Key as json",
                serde_json::to_vec(&self.key).unwrap(),
            );
        });
    }
}
