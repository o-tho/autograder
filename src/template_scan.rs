use crate::image_helpers::{draw_circle_around_box, gray_to_rgb, replace_colour, replace_colours};
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
const BLACK: image::Rgb<u8> = image::Rgb([0u8, 0u8, 0u8]);
const WHITE: image::Rgb<u8> = image::Rgb([255u8, 255u8, 255u8]);

pub struct ColourScheme {
    pub correct_foreground: image::Rgb<u8>,
    pub correct_background: image::Rgb<u8>,
    pub wrong_foreground: image::Rgb<u8>,
    pub wrong_background: image::Rgb<u8>,
    pub highlight_background: image::Rgb<u8>,
    pub highlight_foreground: image::Rgb<u8>,
}

const STD_COLOUR_SCHEME: ColourScheme = ColourScheme {
    correct_foreground: image::Rgb([30u8, 220u8, 30u8]),
    correct_background: image::Rgb([220u8, 255u8, 220u8]),
    wrong_foreground: image::Rgb([255u8, 50u8, 50u8]),
    wrong_background: image::Rgb([255u8, 220u8, 220u8]),
    highlight_background: image::Rgb([255u8, 215u8, 0u8]),
    highlight_foreground: image::Rgb([255u8, 110u8, 0u8]),
};

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
        if let Some(vq) = &t.version {
            all_questions.push(vq.clone());
        }
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

        if let Some(vq) = &t.version {
            println!("Version at ({:#?}):", trafo(vq.boxes[0].a));
            let blacknesses: Vec<u32> = vq.blacknesses_rounded(self);
            println!("{:?} -> {:?}", blacknesses, vq.choice(self));
        }
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
    fn mark_issue_with_highlight_box(
        &self,
        image: &mut image::RgbImage,
        question: &crate::template::Question,
        trafo: impl Fn(Point) -> Point,
        scheme: &ColourScheme,
    ) {
        let a = trafo(question.boxes[0].a);
        let b = trafo(question.boxes.last().unwrap().b);

        replace_colour(
            image,
            a.x,
            a.y,
            b.x,
            b.y,
            WHITE,
            scheme.highlight_background,
        );
    }

    pub fn generate_image_report(&self, k: &ExamKey, identifier: &String) -> ImageReport {
        let t = &self.template;
        let mut image = gray_to_rgb(&self.scan.image);
        let mut scores: Vec<Option<u32>> = vec![];
        let mut issue = false;
        let scheme = STD_COLOUR_SCHEME;

        let trafo = |p| self.transform(p);

        // draw the circle centers
        for c in t.circle_centers {
            let coord = trafo(c);
            drawing::draw_cross_mut(
                &mut image,
                scheme.highlight_foreground,
                coord.x as i32,
                coord.y as i32,
            );
        }

        let v = if let Some(vq) = &t.version {
            vq.choice(self)
        } else {
            Some(0)
        };

        if let Some(v) = v {
            if let Some(vq) = &t.version {
                let thebox = vq.boxes[v as usize];
                replace_colour(
                    &mut image,
                    trafo(thebox.a).x,
                    trafo(thebox.a).y,
                    trafo(thebox.b).x,
                    trafo(thebox.b).y,
                    BLACK,
                    scheme.highlight_foreground,
                );
            }

            let pad_v = if t.questions.len() > 1 {
                let b = trafo(t.questions[0].boxes[0].b);
                let a = trafo(t.questions[1].boxes[0].a);
                (a.y - b.y) / 2
            } else {
                0
            };

            let pad_h = {
                let b = trafo(t.questions[0].boxes[0].b);
                let a = trafo(t.questions[0].boxes[1].a);
                (a.x - b.x) / 2
            };

            for i in 0..t.questions.len() {
                let mut score = None;
                let q = &t.questions[i];
                let choices = q.choices(self);
                let correct_answer = &k[v as usize][i];

                if choices.len() == 1 {
                    if correct_answer.correct(choices[0]) {
                        score = Some(1);
                    } else {
                        score = Some(0);
                    }
                }
                scores.push(score);

                if choices.len() > 1 {
                    for correct in correct_answer.iter() {
                        let a = trafo(q.boxes[correct as usize].a);
                        let b = trafo(q.boxes[correct as usize].b);
                        replace_colour(
                            &mut image,
                            a.x - pad_h,
                            a.y - pad_v,
                            b.x + pad_h,
                            b.y + pad_v,
                            WHITE,
                            scheme.correct_background,
                        );
                    }
                    let a = trafo(q.boxes[0].a);
                    let b = trafo(q.boxes.last().unwrap().b);
                    replace_colour(
                        &mut image,
                        a.x - pad_h,
                        a.y - pad_v,
                        b.x + pad_h,
                        b.y + pad_v,
                        WHITE,
                        scheme.highlight_background,
                    );
                    issue = true;

                    for choice in choices {
                        let thebox = q.boxes[choice as usize];
                        let a = trafo(thebox.a);
                        let b = trafo(thebox.b);
                        replace_colour(
                            &mut image,
                            a.x - pad_h,
                            a.y - pad_v,
                            b.x + pad_h,
                            b.y + pad_v,
                            BLACK,
                            scheme.highlight_foreground,
                        );
                    }
                    continue;
                }

                for (idx, thebox) in q.boxes.iter().enumerate() {
                    let mut replacements = std::collections::HashMap::new();

                    if correct_answer.correct(idx as u32) {
                        replacements.insert(WHITE, scheme.correct_background);

                        if choices.contains(&(idx as u32)) {
                            replacements.insert(BLACK, scheme.correct_foreground);
                        }
                    } else {
                        replacements.insert(WHITE, scheme.wrong_background);

                        if choices.contains(&(idx as u32)) {
                            replacements.insert(BLACK, scheme.wrong_foreground);
                        }
                    }
                    let a = trafo(thebox.a);
                    let b = trafo(thebox.b);

                    replace_colours(
                        &mut image,
                        a.x - pad_h,
                        a.y - pad_v,
                        b.x + pad_h,
                        b.y + pad_v,
                        replacements,
                    );
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
                        let a = trafo(prev_question.boxes[0].a);
                        let b = trafo(prev_question.boxes.last().unwrap().b);

                        replace_colour(
                            &mut image,
                            a.x,
                            a.y,
                            b.x,
                            b.y,
                            WHITE,
                            scheme.highlight_background,
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
                    replace_colour(
                        &mut image,
                        tl.x,
                        tl.y,
                        br.x,
                        br.y,
                        BLACK,
                        scheme.highlight_foreground,
                    );
                }
                n if n > 1 => {
                    self.mark_issue_with_highlight_box(&mut image, q, trafo, &scheme);
                    issue = true;
                }
                _ => {}
            }
        }

        ImageReport {
            image,
            sid: self.id(),
            version: v,
            scores,
            issue,
            identifier: identifier.to_string(),
        }
    }

    fn set_transformation(&mut self) {
        let trafo = self.find_transformation();

        if trafo.is_some() {
            self.transformation = trafo;
            return;
        }

        // if this did not work, then in all cases known to us the image had too
        // much white noise, so we erode it
        imageproc::morphology::erode_mut(
            &mut self.scan.image,
            imageproc::distance_transform::Norm::L1,
            1,
        );

        self.transformation = self.find_transformation();
    }

    fn find_transformation(&self) -> Option<Transformation> {
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
