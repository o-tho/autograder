#[cfg(not(target_arch = "wasm32"))]
use autograder::ErrorWrapper;
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), ErrorWrapper> {
    use autograder::debug_report;
    use autograder::generate_reports_for_image_container;
    use autograder::image_container::{PdfContainer, SingleImageContainer, TiffContainer};
    use autograder::template::{ExamKey, Template};
    use clap::{Arg, Command};
    use std::path::Path;
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
            Command::new("debug")
                .about("Run in debug mode")
                .arg(
                    Arg::new("template")
                        .default_value("tests/assets/template.json")
                        .help("template configuration"),
                )
                .arg(Arg::new("image").help("single image to be debugged")),
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
                    let file = pdf::file::FileOptions::cached().open(imagefile).unwrap();
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
        Some(("debug", sub_matches)) => {
            let templatepath = sub_matches
                .get_one::<String>("template")
                .unwrap()
                .to_string();

            let imagepath = sub_matches.get_one::<String>("image").unwrap().to_string();

            let t: Template = serde_json::from_reader(std::fs::File::open(templatepath)?)?;

            let imagefile = Path::new(&imagepath);
            let image = image::open(imagefile)?;

            let mut container = SingleImageContainer { image };

            debug_report(&mut container, &t);
        }
        _ => println!("Please specify a valid subcommand (e.g., `report` or `debug`)."),
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use autograder::webapp::WebApp;
    use eframe::wasm_bindgen::JsCast;
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");
        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|_cc| Ok(Box::new(WebApp::default()))),
            )
            .await;
        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
