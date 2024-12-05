use crate::point::Point;
use crate::scan::Scan;
use serde::{Deserialize, Serialize};

const THRESHOLD: f64 = 0.30;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Template {
    pub id_questions: Vec<Question>,
    pub version: Question,
    pub questions: Vec<Question>,
    pub circle_centers: [Point; 3],
    pub circle_radius: u32,
    pub height: u32,
    pub width: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Question {
    pub id: String,
    pub boxes: Vec<Box>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Box {
    pub a: Point,
    pub b: Point,
    pub value: u32,
}
impl Box {
    pub fn checked(self, scan: &Scan) -> bool {
        self.blackness(scan) > THRESHOLD
    }
    pub fn blackness(&self, scan: &Scan) -> f64 {
        let a: Point;
        let b: Point;

        if let Some(trafo) = scan.transformation {
            a = trafo.apply(self.a);
            b = trafo.apply(self.b);
        } else {
            a = self.a;
            b = self.b;
        }

        scan.blackness(a, b)
    }
}

impl Question {
    pub fn blacknesses(&self, scan: &Scan) -> Vec<f64> {
        self.boxes
            .clone()
            .into_iter()
            .map(|b| b.blackness(scan))
            .collect()
    }

    pub fn blacknesses_rounded(&self, scan: &Scan) -> Vec<u32> {
        self.blacknesses(scan)
            .into_iter()
            .map(|b| (b * 100.0).round() as u32)
            .collect()
    }
    pub fn choices(&self, scan: &Scan) -> Vec<u32> {
        let mut choices = Vec::new();
        let blackness: Vec<f64> = self
            .blacknesses(scan)
            .iter()
            .map(|&v| if v.is_nan() { 0.0 } else { v })
            .collect();

        let (min, max) = blackness
            .iter()
            .copied()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), v| {
                (min.min(v), max.max(v))
            });

        let beta = 0.6;
        let threshold = min + beta * (max - min);

        // Find all boxes above threshold
        for (index, &value) in blackness.iter().enumerate() {
            if value > threshold && max > 0.45 {
                choices.push(self.boxes[index].value);
            }
        }

        choices
    }
    pub fn choice(&self, scan: &Scan) -> Option<u32> {
        let choices = self.choices(scan);
        if choices.len() == 1 {
            Some(choices[0])
        } else {
            None
        }
    }
}

pub type ExamKey = Vec<Vec<u32>>;

// check whether template and key are compatible: the number of versions needs
// to match and every version needs to have answers for all questions.
pub fn are_compatible(t: &Template, k: &ExamKey) -> bool {
    if k.len() != t.version.boxes.len() {
        return false;
    }

    if let Some(first_len) = k.first().map(|v| v.len()) {
        k.iter().all(|v| v.len() == first_len) && first_len == t.questions.len()
    } else {
        false
    }
}
