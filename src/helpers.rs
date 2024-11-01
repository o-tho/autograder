#![allow(dead_code)]
use crate::point::Point;
use crate::scan::{binary_image_from_file, Scan};
use crate::template::{Box, Question, Template};
use std::fs::File;

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

fn gen_std_template() -> Result<(), serde_json::Error> {
    let qs = 20;
    let idqs = 9;
    let h = 41;
    let w = 45;
    let pad_h = 15;
    let pad_v = 7;
    let vs = 4;
    let answers = 5;

    let first_q = Point { x: 483, y: 400 };
    let first_vq = Point { x: 1404, y: 784 };
    let first_idq = Point { x: 1404, y: 928 };

    let t = Template {
        id_questions: question_builder(first_idq, 10, w, h, pad_h, pad_v, idqs, &"id.".to_string()),
        version: Question {
            id: "version".to_string(),
            boxes: box_builder(first_vq, w, h, pad_h, vs),
        },
        questions: question_builder(first_q, answers, w, h, pad_h, pad_v, qs, &"q.".to_string()),
        circle_centers: [
            Point { x: 293, y: 269 },
            Point { x: 2240, y: 270 },
            Point { x: 2240, y: 3116 },
        ],
        circle_radius: 47,
        height: 3487,
        width: 2468,
    };

    let json = serde_json::to_string_pretty(&t)?;
    println!("{}", json);

    Ok(())
}

fn trafo() -> Result<(), std::boxed::Box<dyn std::error::Error>> {
    let img = binary_image_from_file(&"tests/assets/example.png".to_string());
    let t: Template = serde_json::from_reader(File::open("tests/assets/template.json")?)?;

    let mut scan = Scan {
        img,
        transformation: None,
    };

    let trafo = scan.find_transformation(&t).ok_or("no trafo found")?;
    scan.transformation = Some(trafo);

    let q15 = &t.questions[14];

    let real_coord_of_c = trafo.apply(q15.boxes[2].a);

    println!("{} -> {}", q15.boxes[2].a, real_coord_of_c);

    Ok(())
}
fn find_centers() -> Result<(), std::boxed::Box<dyn std::error::Error>> {
    let img = binary_image_from_file(&"tests/assets/template-a4-weirdscale.png".to_string());

    let approx_centers = [
        Point { x: 292, y: 271 },
        Point { x: 2240, y: 267 },
        Point { x: 2227, y: 3113 },
    ];
    let approx_r = 47;

    let scan = Scan {
        img,
        transformation: None,
    };

    for p in approx_centers {
        let c = scan.real_center(p, approx_r).expect("no center found");
        println!("{}", c);
    }

    Ok(())
}

fn number_to_letter(n: u32) -> char {
    (b'A' + (n as u8)) as char
}

fn example() -> Result<(), std::boxed::Box<dyn std::error::Error>> {
    let img = binary_image_from_file(&"tests/assets/example.png".to_string());
    let t: Template = serde_json::from_reader(File::open("tests/assets/template.json")?)?;

    let mut scan = Scan {
        img,
        transformation: None,
    };

    let trafo = scan.find_transformation(&t);
    scan.transformation = trafo;

    println!("{:?}", scan.id(&t));

    // for q in t.questions {
    //     println!("{}: {:?}", q.id, q.choice(&scan));
    // }

    Ok(())
}
