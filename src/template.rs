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
            .map(|b| (b.blackness(&scan) * 100.0).round() / 100.0)
            .collect()
    }

    pub fn blacknesses_rounded(&self, scan: &Scan) -> Vec<u32> {
        self.blacknesses(scan)
            .into_iter()
            .map(|b| (b * 100.0).round() as u32)
            .collect()
    }
    pub fn choice(&self, scan: &Scan) -> Option<u32> {
        let blackness = self.blacknesses(scan);
        if blackness.len() < 2 {
            return None;
        }

        let (min, max) = blackness
            .iter()
            .copied()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), v| {
                (min.min(v), max.max(v))
            });

        if let Some(max_index) = blackness.iter().position(|&v| v == max) {
            let second_highest = blackness
                .iter()
                .copied()
                .filter(|&v| v != max)
                .max_by(|a, b| a.partial_cmp(b).unwrap())?;

            if max > second_highest + min / 2.0 {
                return Some(self.boxes[max_index].value);
            }
        }
        None
    }
}

pub type ExamKey = Vec<Vec<u32>>;
