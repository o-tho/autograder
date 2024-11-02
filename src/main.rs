use autograder::generate_reports_for_pdf;
use autograder::report::ImageReport;
use autograder::scan::{binary_image_from_file, binary_image_from_image, Scan};
use autograder::template::{ExamKey, Template};
use image::{DynamicImage, GrayImage, RgbaImage};
use rayon::prelude::*;
use std::env;
use std::fs::File;
use std::path::Path;

fn score_image(img: GrayImage, t: &Template, k: &ExamKey, comment: &String) -> String {
    let result: String;
    let mut scan = Scan {
        img,
        transformation: None,
    };
    let trafo = scan.find_transformation(&t);
    scan.transformation = trafo;

    if let Some(id) = scan.id(&t) {
        if let Some(score) = scan.score(&t, &k) {
            result = format!(";{}; {}; {}", comment, id, score);
        } else {
            result = format!("no valid version; {}; {};", comment, id);
        }
    } else {
        result = format!("cannot read id;{};;", comment);
    }
    result
}

fn score_file(f: &String, t: &Template, k: &ExamKey) -> String {
    let img = binary_image_from_file(&f);
    score_image(img, &t, &k, &f)
}

// fn score_file_image_report(f: &String, t: &Template, k: &ExamKey) {
//     let img = binary_image_from_file(&f);
//     let mut scan = Scan {
//         img,
//         transformation: None,
//     };
//     let trafo = scan.find_transformation(&t);
//     scan.transformation = trafo;

//     let result = scan.score_report_as_image(&t, &k);

//     result.save_with_format("out.png", image::ImageFormat::Png);
// }

fn debug_file(f: &String, t: &Template, k: &ExamKey) {
    let img = binary_image_from_file(&f);
    let mut scan = Scan {
        img,
        transformation: None,
    };
    let trafo = scan.find_transformation(&t);
    scan.transformation = trafo;

    let vblackness: Vec<f64> = t.version.boxes.iter().map(|b| b.blackness(&scan)).collect();
    println!(
        "found id {:?} and version {:?}",
        scan.id(&t),
        t.version.choice(&scan)
    );
    println!("version blackness: {:?}", vblackness);

    for i in 0..t.questions.len() {
        let q = &t.questions[i as usize];
        let blackness: Vec<f64> = q.boxes.iter().map(|b| b.blackness(&scan)).collect();
        println!("question {} has blackness {:?}", i, blackness);
    }
}

// fn testing() -> Result<(), Box<dyn std::error::Error>> {
//     // Collect command-line arguments
//     let args: Vec<String> = env::args().collect();
//     if args.len() < 3 {
//         eprintln!(
//             "Usage: {} <template.json> <examkey.json> <scan1.png> <scan2.png> ...",
//             args[0]
//         );
//         std::process::exit(1);
//     }

//     // Load the Template from the first argument
//     let template_path = Path::new(&args[1]);
//     let t: Template = serde_json::from_reader(File::open(template_path)?)?;

//     // Load the ExamKey from the second argument
//     let exam_key_path = Path::new(&args[2]);
//     let k: ExamKey = serde_json::from_reader(File::open(exam_key_path)?)?;

//     // Collect remaining arguments as filenames
//     let filenames: Vec<String> = args[3..].to_vec();

//     filenames
//         .iter()
//         .for_each(|f| score_file_image_report(f, &t, &k));
//     //.for_each(|f| debug_file(f, &t, &k));

//     Ok(())
// }
fn actual() -> Result<(), Box<dyn std::error::Error>> {
    // Collect command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <template.json> <examkey.json> <scan1.png> <scan2.png> ...",
            args[0]
        );
        std::process::exit(1);
    }

    // Load the Template from the first argument
    let template_path = Path::new(&args[1]);
    let t: Template = serde_json::from_reader(File::open(template_path)?)?;

    // Load the ExamKey from the second argument
    let exam_key_path = Path::new(&args[2]);
    let k: ExamKey = serde_json::from_reader(File::open(exam_key_path)?)?;

    // Collect remaining arguments as filenames
    let filenames: Vec<String> = args[3..].to_vec();

    // Iterate over filenames
    let res: Vec<String> = filenames
        .par_iter()
        .map(|f| score_file(&f, &t, &k))
        .collect();

    for s in res {
        println!("{}", s);
    }

    Ok(())
}
fn main() -> Result<(), std::boxed::Box<dyn std::error::Error>> {
    // Collect command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <template.json> <examkey.json> <scans.pdf>",
            args[0]
        );
        std::process::exit(1);
    }

    let _ = generate_reports_for_pdf(
        args[3].to_string(),
        args[1].to_string(),
        args[2].to_string(),
    );
    // Load the Template from the first argument
    Ok(())
}
