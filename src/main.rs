#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), std::boxed::Box<dyn std::error::Error>> {
    use autograder::debug_report;
    use autograder::generate_reports_for_image_container;
    use autograder::image_container::{PdfContainer, SingleImageContainer, TiffContainer};
    use autograder::template::{ExamKey, Template};
    use autograder::typst_helpers::typst_frame_to_template;
    use autograder::typst_helpers::*;
    use clap::{value_parser, Arg, Command};
    use std::path::Path;
    let matches = Command::new("autograder")
        .about("automatically grade MCQ exams using optical mark recognition")
        .subcommand(Command::new("test"))
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
        .subcommand(
            Command::new("form")
                .about("Generate a form and output the PDF, PNG and template.json")
                .arg(
                    Arg::new("qs")
                        .long("qs")
                        .required(true)
                        .value_parser(value_parser!(u32))
                        .help("Number of questions"),
                )
                .arg(
                    Arg::new("idqs")
                        .long("idqs")
                        .required(true)
                        .value_parser(value_parser!(u32))
                        .help("Number of ID questions"),
                )
                .arg(
                    Arg::new("versions")
                        .long("versions")
                        .required(true)
                        .value_parser(value_parser!(u32))
                        .help("Number of versions"),
                )
                .arg(
                    Arg::new("choices")
                        .long("choices")
                        .required(true)
                        .value_parser(value_parser!(u32))
                        .help("Number of choices in each MCQ"),
                )
                .arg(
                    Arg::new("outprefix")
                        .long("outprefix")
                        .value_name("PREFIX")
                        .default_value("form")
                        .help("Specify the output file prefix"),
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

            let t: Template = serde_json::from_reader(
                std::fs::File::open(templatepath).expect("could not open template"),
            )?;
            let k: ExamKey =
                serde_json::from_reader(std::fs::File::open(keypath).expect("could not open key"))?;

            let imagefile = Path::new(&imagespath);

            let mut csv_report = String::new();

            match imagefile.extension().and_then(|ext| ext.to_str()) {
                Some("pdf") => {
                    let file = pdf::file::FileOptions::cached().open(imagefile).unwrap();
                    let mut container = PdfContainer { pdf_file: file };

                    csv_report =
                        generate_reports_for_image_container(&mut container, &t, &k, outpath)
                            .expect("error generating report");
                    println!("{}", csv_report);
                }
                Some("tif") | Some("tiff") => {
                    let buffer = std::io::BufReader::new(
                        std::fs::File::open(imagefile).expect("could not open tif container"),
                    );

                    let tiff = tiff::decoder::Decoder::new(buffer)?;

                    let mut container = TiffContainer { decoder: tiff };

                    csv_report =
                        generate_reports_for_image_container(&mut container, &t, &k, outpath)
                            .expect("error while generating report");
                }
                Some("jpg") | Some("jpeg") | Some("png") => {
                    let image = image::open(imagefile).expect("could not open single image");

                    let mut container = SingleImageContainer { image };

                    let _ = generate_reports_for_image_container(&mut container, &t, &k, outpath)
                        .expect("error while generating report");
                }
                _ => println!("Unsupported file type: {:?}", imagefile),
            }

            if !csv_report.is_empty() {
                println!("{}", csv_report);
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

            let container = SingleImageContainer { image };

            debug_report(&container, &t);
        }
        Some(("form", sub_matches)) => {
            let qs = sub_matches.get_one::<u32>("qs").expect("required by clap");
            let idqs = sub_matches
                .get_one::<u32>("idqs")
                .expect("required by clap");
            let versions = sub_matches
                .get_one::<u32>("versions")
                .expect("required by clap");
            let choices = sub_matches
                .get_one::<u32>("choices")
                .expect("required by clap");
            let outprefix = sub_matches
                .get_one::<String>("outprefix")
                .expect("defaulted by clap");
            let input = std::fs::read_to_string("assets/formtemplate.typ")?;

            let code = format!(
                r#"
#let num_qs = {}
#let num_idqs = {}
#let num_answers = {}
#let num_versions = {}
{}
"#,
                qs, idqs, choices, versions, input
            );

            let wrapper = TypstWrapper::new(code);

            let document = typst::compile(&wrapper)
                .output
                .expect("Error from Typst. This really should not happen. So sorry.");

            let scale = 3.0;
            let template = typst_frame_to_template(&document.pages[0].frame, scale);

            let _ = serde_json::to_writer_pretty(
                &std::fs::File::create(outprefix.to_owned() + ".json").unwrap(),
                &template,
            );

            // println!("{:#?}", document);

            let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).expect("bla");
            let _ = std::fs::write(format!("{}.pdf", outprefix), pdf);

            let _ = typst_render::render(&document.pages[0], scale as f32)
                .save_png(outprefix.to_owned() + ".png");
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
