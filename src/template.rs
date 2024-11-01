use crate::point::Point;
use crate::scan::Scan;
use serde::{Deserialize, Serialize};

const THRESHOLD: f64 = 0.4;

#[derive(Debug, Serialize, Deserialize)]
pub struct Template {
    pub id_questions: Vec<Question>,
    pub version: Question,
    pub questions: Vec<Question>,
    pub circle_centers: [Point; 3],
    pub circle_radius: u32,
    pub height: u32,
    pub width: u32,
}

#[derive(Debug, Serialize, Deserialize)]
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
        let a: Point;
        let b: Point;

        if let Some(trafo) = scan.transformation {
            a = trafo.apply(self.a);
            b = trafo.apply(self.b);
        } else {
            a = self.a;
            b = self.b;
        }

        scan.blackness(a, b) > THRESHOLD
    }
}

impl Question {
    pub fn choice(&self, scan: &Scan) -> Option<u32> {
        let choices: Vec<&Box> = self.boxes.iter().filter(|b| b.checked(scan)).collect();
        if choices.len() == 1 {
            Some(choices[0].value)
        } else {
            None
        }
    }
}

pub type ExamKey = Vec<Vec<u32>>;
