use autograder::generate_reports_for_image_container;
use autograder::image_container::{PdfContainer, SingleImageContainer, TiffContainer};
use autograder::image_helpers::binary_image_from_file;
use autograder::scan::Scan;
use autograder::template::{ExamKey, Template};
use autograder::ErrorWrapper;
use clap::{Arg, Command};
use image::GrayImage;
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
fn main() -> Result<(), ErrorWrapper> {
    let matches = Command::new("autograder")
        .about("automatically grade MCQ exams using optical mark recognition")
        .subcommand(
            Command::new("report")
                .about("Generate a report")
                .arg(
                    Arg::new("outpath")
                        .long("outpath")
                        .value_name("OUT")
                        .default_value("./")
                        .help("Specify the output path"),
                )
                .arg(
                    Arg::new("template")
                        .default_value("tests/assets/template.json")
                        .help("template configuration"),
                )
                .arg(
                    Arg::new("key")
                        .default_value("tests/assets/key.json")
                        .help("exam key"),
                )
                .arg(
                    Arg::new("images")
                        .default_value("tests/assets/scanner-multipagetiff.tif")
                        .help("image container in PDF or multipage TIFF format"),
                ),
        )
        .subcommand(
            Command::new("debug").about("Run in debug mode").arg(
                Arg::new("config")
                    .long("config")
                    .value_name("CONFIG")
                    .help("Specify the debug configuration file"),
            ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("report", sub_matches)) => {
            let outpath = sub_matches
                .get_one::<String>("outpath")
                .unwrap()
                .to_string();
            let templatepath = sub_matches
                .get_one::<String>("template")
                .unwrap()
                .to_string();

            let keypath = sub_matches.get_one::<String>("key").unwrap().to_string();
            let imagespath = sub_matches.get_one::<String>("images").unwrap().to_string();

            let t: Template = serde_json::from_reader(std::fs::File::open(templatepath)?)?;
            let k: ExamKey = serde_json::from_reader(std::fs::File::open(keypath)?)?;

            let imagefile = Path::new(&imagespath);

            match imagefile.extension().and_then(|ext| ext.to_str()) {
                Some("pdf") => {
                    let file = pdf::file::FileOptions::uncached().open(imagefile).unwrap();
                    let mut container = PdfContainer { pdf_file: file };

                    generate_reports_for_image_container(&mut container, &t, &k, outpath)?
                }
                Some("tif") | Some("tiff") => {
                    let buffer = std::io::BufReader::new(std::fs::File::open(imagefile)?);

                    let tiff = tiff::decoder::Decoder::new(buffer)?;

                    let mut container = TiffContainer { decoder: tiff };

                    generate_reports_for_image_container(&mut container, &t, &k, outpath)?
                }
                Some("jpg") | Some("jpeg") | Some("png") => {
                    let image = image::open(imagefile)?;

                    let mut container = SingleImageContainer { image };

                    generate_reports_for_image_container(&mut container, &t, &k, outpath)?
                }
                _ => println!("Unsupported file type: {:?}", imagefile),
            }
        }
        Some(("debug", _sub_matches)) => {
            println!("Not implemented");
        }
        _ => println!("Please specify a valid subcommand (e.g., `report` or `debug`)."),
    }
    // Load the Template from the first argument
    Ok(())
}
