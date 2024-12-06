use crate::image_helpers::{draw_circle_around_box, draw_rectangle_around_box, gray_to_rgb};
use crate::point::affine_transformation;
use crate::point::Point;
use crate::point::Transformation;
use crate::report::ImageReport;
use crate::scan::Scan;
use crate::template::ExamKey;
use crate::template::Template;

use imageproc::drawing;

const RED: image::Rgb<u8> = image::Rgb([255u8, 0u8, 0u8]);
const GREEN: image::Rgb<u8> = image::Rgb([0u8, 255u8, 0u8]);
const ORANGE: image::Rgb<u8> = image::Rgb([255u8, 140u8, 0u8]);

pub struct TemplateScan<'a> {
    pub template: &'a Template,
    pub scan: Scan,
    pub transformation: Option<Transformation>,
}

impl<'a> TemplateScan<'a> {
    pub fn new(template: &'a Template, scan: Scan) -> Self {
        let mut ts = TemplateScan {
            template,
            scan,
            transformation: None,
        };
        ts.set_transformation();
        ts
    }
    pub fn transform(&self, p: Point) -> Point {
        if let Some(trafo) = self.transformation {
            trafo.apply(p)
        } else {
            p
        }
    }
    pub fn id(&self) -> Option<u32> {
        let t = &self.template;
        let choices: Vec<Option<u32>> = t.id_questions.iter().map(|q| q.choice(self)).collect();

        let id: String = choices
            .iter()
            .filter_map(|&opt| opt.map(|num| num.to_string()))
            .collect();

        // If the resulting string is empty, all entries were None, so return None
        if id.is_empty() {
            None
        } else {
            id.parse::<u32>().ok()
        }
    }
    pub fn score_against(&self, k: &ExamKey) -> Option<u32> {
        let mut score = 0;
        let t = &self.template;

        if let Some(v) = t.version.choice(self) {
            for i in 0..t.questions.len() {
                let q = &t.questions[i];
                if let Some(answer) = q.choice(self) {
                    if answer == k[v as usize][i] {
                        score += 1;
                    }
                }
            }
            Some(score)
        } else {
            None
        }
    }

    pub fn circle_everything(&self) -> image::RgbImage {
        let t = &self.template;
        let mut image = gray_to_rgb(&self.scan.image);

        let trafo = |p| self.transform(p);
        for c in t.circle_centers {
            let coord = trafo(c);
            drawing::draw_cross_mut(&mut image, RED, coord.x as i32, coord.y as i32);
            for i in 0..4 {
                drawing::draw_hollow_circle_mut(
                    &mut image,
                    (coord.x as i32, coord.y as i32),
                    (t.circle_radius + i) as i32,
                    RED,
                );
            }
        }

        let mut all_questions = t.questions.clone();
        all_questions.push(t.version.clone());
        all_questions.extend(t.id_questions.clone());

        for q in all_questions {
            for b in q.boxes {
                draw_circle_around_box(&mut image, b.a, b.b, GREEN);
            }
        }

        image
    }

    pub fn debug_report(&self) {
        let t = &self.template;
        println!("Generating debugging report ...");

        let trafo = |p| self.transform(p);
        println!("Found centers at {:#?}", t.circle_centers.map(&trafo));

        println!("Version at ({:#?}):", trafo(t.version.boxes[0].a));

        let blacknesses: Vec<u32> = t.version.blacknesses_rounded(self);
        println!("{:?} -> {:?}", blacknesses, t.version.choice(self));

        println!("\nID Questions:");

        for (idx, q) in t.id_questions.clone().into_iter().enumerate() {
            let blacknesses: Vec<u32> = q.blacknesses_rounded(self);
            println!("ID{}: {:?} -> {:?}", idx + 1, blacknesses, q.choice(self));
        }

        println!("\nMCQ:");

        for (idx, q) in t.questions.clone().into_iter().enumerate() {
            let blacknesses: Vec<u32> = q.blacknesses_rounded(self);
            println!(
                "Q{:0>2}: {:?} -> {:?}",
                idx + 1,
                blacknesses,
                q.choices(self)
            );
        }
    }
    pub fn generate_image_report(&self, k: &ExamKey, identifier: &String) -> ImageReport {
        let t = &self.template;
        let mut image = gray_to_rgb(&self.scan.image);
        let mut score = 0;
        let mut issue = false;

        let trafo = |p| self.transform(p);

        // draw the circle centers
        for c in t.circle_centers {
            let coord = trafo(c);
            drawing::draw_cross_mut(&mut image, RED, coord.x as i32, coord.y as i32);
        }

        if let Some(v) = t.version.choice(self) {
            let thebox = t.version.boxes[v as usize];
            draw_circle_around_box(&mut image, trafo(thebox.a), trafo(thebox.b), GREEN);

            for i in 0..t.questions.len() {
                let q = &t.questions[i];
                let correct = k[v as usize][i] as usize;
                let choices = q.choices(self);
                let correct_box_a = trafo(q.boxes[correct].a);
                let correct_box_b = trafo(q.boxes[correct].b);
                match choices.len() {
                    0 => {
                        draw_circle_around_box(&mut image, correct_box_a, correct_box_b, RED);
                    }
                    1 => {
                        let color = if choices[0] as usize == correct {
                            score += 1;
                            GREEN
                        } else {
                            RED
                        };
                        draw_circle_around_box(&mut image, correct_box_a, correct_box_b, color);
                    }
                    _ => {
                        draw_circle_around_box(&mut image, correct_box_a, correct_box_b, RED);
                        draw_rectangle_around_box(
                            &mut image,
                            trafo(q.boxes.first().unwrap().a),
                            trafo(q.boxes.last().unwrap().b),
                            ORANGE,
                        );
                        issue = true;
                    }
                }
            }
        }

        let mut last_valid_id_pos = None;
        for i in 0..t.id_questions.len() {
            let q = &t.id_questions[i];
            let choices = q.choices(self);

            if !choices.is_empty() {
                // If we found a previous valid position and there's a gap
                if let Some(last_pos) = last_valid_id_pos {
                    if i - last_pos > 1 {
                        issue = true;
                        let prev_question = &t.id_questions[i - 1];
                        draw_rectangle_around_box(
                            &mut image,
                            trafo(prev_question.boxes[0].a),
                            trafo(prev_question.boxes.last().unwrap().b),
                            ORANGE,
                        );
                    }
                }
                last_valid_id_pos = Some(i);
            }

            match choices.len() {
                1 => {
                    let idx = choices[0];
                    let tl = trafo(q.boxes[idx as usize].a);
                    let br = trafo(q.boxes[idx as usize].b);
                    draw_circle_around_box(&mut image, tl, br, GREEN);
                }
                n if n > 1 => {
                    draw_rectangle_around_box(
                        &mut image,
                        trafo(q.boxes[0].a),
                        trafo(q.boxes.last().unwrap().b),
                        ORANGE,
                    );
                    issue = true;
                }
                _ => {}
            }
        }

        ImageReport {
            image,
            sid: self.id(),
            version: t.version.choice(self),
            score,
            issue,
            identifier: identifier.to_string(),
        }
    }

    pub fn set_transformation(&mut self) {
        let trafo = self.find_transformation();
        self.transformation = trafo;
    }

    pub fn find_transformation(&self) -> Option<Transformation> {
        let t = &self.template;
        let h_scale = (t.height as f64) / (self.scan.image.height() as f64);
        let w_scale = (t.width as f64) / (self.scan.image.width() as f64);

        let scale = (h_scale + w_scale) / 2.0;

        let projected_centers = t.circle_centers.map(|p| Point {
            x: (p.x as f64 / scale).round() as u32,
            y: (p.y as f64 / scale).round() as u32,
        });

        let projected_radius = (t.circle_radius as f64 / scale * 1.05).round() as u32;

        let located_centers: Option<Vec<Point>> = projected_centers
            .iter()
            .map(|p| self.scan.real_center_fuzzy(*p, projected_radius))
            .collect();

        match located_centers {
            Some(centers) => affine_transformation(
                t.circle_centers[0],
                t.circle_centers[1],
                t.circle_centers[2],
                centers[0],
                centers[1],
                centers[2],
            ),
            None => None,
        }
    }
}
