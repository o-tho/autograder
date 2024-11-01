mod helpers;
mod point;
mod scan;
mod template;

use scan::{binary_image_from_file, Scan};
use std::env;
use std::fs::File;
use std::path::Path;
use template::{ExamKey, Template};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    for f in filenames {
        println!("Processing file: {}", f);
        let img = binary_image_from_file(&f);
        let mut scan = Scan {
            img,
            transformation: None,
        };
        let trafo = scan.find_transformation(&t);
        scan.transformation = trafo;

        if let Some(id) = scan.id(&t) {
            if let Some(score) = scan.score(&t, &k) {
                println!("{}, {}", id, score);
            } else {
                println!("{}, no valid version", id);
            }
        } else {
            println!("cannot read ID");
        }
    }

    Ok(())
}
