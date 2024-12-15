use autograder::image_helpers::binary_image_from_image;
use autograder::template::{are_compatible, ExamKey, Question, Template};
use autograder::typst_helpers::generate_form_and_template;
use imageproc::drawing;
use itertools::Itertools;

const BLACK: image::Rgba<u8> = image::Rgba([0u8, 0u8, 0u8, 255u8]);
fn box_to_center_and_radius(b: &autograder::template::Box) -> ((i32, i32), i32) {
    let radius = (b.b.x - b.a.x) / 2;
    let x = (b.b.x + b.a.x) / 2;
    let y = (b.b.y + b.a.y) / 2;
    ((x as i32, y as i32), radius as i32)
}

fn check_box(image: &mut image::DynamicImage, b: &autograder::template::Box) {
    let (center, radius) = box_to_center_and_radius(b);
    drawing::draw_filled_circle_mut(image, center, radius, BLACK);
}

fn fill_out(
    image: &image::DynamicImage,
    template: &Template,
    id: u32,
    version: u32,
    choices: Vec<u32>,
) -> image::DynamicImage {
    let mut result = image.clone();
    let id: Vec<u32> = id
        .to_string()
        .chars()
        .map(|d| d.to_digit(10).unwrap())
        .collect();

    // filling out the version
    let thebox = template
        .version
        .boxes
        .iter()
        .find(|&b| b.value == version)
        .unwrap();

    check_box(&mut result, thebox);

    // filling out the IDs
    let id_qs: Vec<Question> = template
        .id_questions
        .iter()
        .sorted_by(|a, b| {
            let a_box = a.boxes.first().unwrap();
            let b_box = b.boxes.first().unwrap();
            a_box.a.x.cmp(&b_box.a.x).then(a_box.a.y.cmp(&b_box.a.y))
        })
        .cloned()
        .collect();

    for (idx, d) in id.iter().enumerate() {
        let thebox = id_qs[idx].boxes.iter().find(|&b| b.value == *d).unwrap();
        check_box(&mut result, thebox);
    }

    // filling out the questions
    let qs: Vec<Question> = template
        .questions
        .iter()
        .sorted_by(|a, b| {
            let a_box = a.boxes.first().unwrap();
            let b_box = b.boxes.first().unwrap();
            a_box.a.y.cmp(&b_box.a.y).then(a_box.a.x.cmp(&b_box.a.x))
        })
        .cloned()
        .collect();

    for (idx, c) in choices.iter().enumerate() {
        let thebox = qs[idx].boxes.iter().find(|&b| b.value == *c).unwrap();
        check_box(&mut result, thebox);
    }

    result
}

#[test]
fn generate_form_and_grade() {
    // we first create a form
    let (document, template) = generate_form_and_template(5, 10, 4, 5, 3.0);

    let _ =
        typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).expect("typst to pdf error");
    let png_stream = typst_render::render(&document.pages[0], 3.0)
        .encode_png()
        .expect("typst to png error");

    let form_image =
        image::load_from_memory(&png_stream).expect("could not parse typst png into image crate");

    // rounding errors are fine, but nothing more extreme
    assert!(template.width.abs_diff(form_image.width()) <= 1);
    assert!(template.height.abs_diff(form_image.height()) <= 1);

    let key: ExamKey = vec![vec![0; 5], vec![1; 5], vec![2; 5], vec![0, 1, 2, 3, 4]];
    let incompatible_key_1: ExamKey = vec![vec![0; 5], vec![1; 4], vec![2; 5], vec![0, 1, 2, 3, 4]];
    let incompatible_key_2: ExamKey = vec![
        vec![0; 5],
        vec![1; 5],
        vec![2; 5],
        vec![2; 5],
        vec![0, 1, 2, 3, 4],
    ];

    assert!(are_compatible(&template, &key));
    assert!(!are_compatible(&template, &incompatible_key_1));
    assert!(!are_compatible(&template, &incompatible_key_2));

    // well filled out forms
    let tests = vec![
        (1234567890, 0, vec![0, 1, 2, 3, 4], 1),
        (123456789, 1, vec![1, 1, 2, 2, 3], 2),
        (999999, 2, vec![4, 1, 3, 2, 0], 1),
        (1234554, 3, vec![0, 1, 2, 3, 4], 5),
    ];
    for test in tests {
        let filled_out = fill_out(&form_image, &template, test.0, test.1, test.2);
        let scan = autograder::scan::Scan {
            image: binary_image_from_image(filled_out),
        };
        let template_scan = autograder::template_scan::TemplateScan::new(&template, scan);
        let report = template_scan.generate_image_report(&key, &"".to_string());
        assert_eq!(report.sid, Some(test.0));
        assert_eq!(report.version, Some(test.1));
        assert!(!report.issue);
        assert_eq!(report.score, test.3);
    }

    // badly filled out forms
    let tests = vec![
        vec![
            (1234567890, 0, vec![0, 1, 2, 3, 4]),
            (1234567891, 0, vec![0, 1, 2, 3, 4]),
        ],
        vec![
            (123456789, 1, vec![1, 1, 2, 2, 3]),
            (123456789, 3, vec![1, 1, 2, 2, 3]),
        ],
        vec![
            (999999, 2, vec![4, 1, 3, 2, 0]),
            (999999, 2, vec![4, 1, 3, 3, 0]),
        ],
    ];
    for test in tests {
        let fst = test[0].clone();
        let snd = test[1].clone();

        let tmp = fill_out(&form_image, &template, fst.0, fst.1, fst.2);
        let filled_out = fill_out(&tmp, &template, snd.0, snd.1, snd.2);
        let scan = autograder::scan::Scan {
            image: binary_image_from_image(filled_out),
        };
        let template_scan = autograder::template_scan::TemplateScan::new(&template, scan);
        let report = template_scan.generate_image_report(&key, &"".to_string());
        assert!(report.issue || report.sid.is_none() || report.version.is_none());
    }
}
