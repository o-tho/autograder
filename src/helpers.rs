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

fn fax2grayimage(fax: &[u8], height: u32, width: u32) -> image::GrayImage {
    let mut img = image::GrayImage::new(width + 1, height + 1);
    for x in 1..(width) {
        for y in 1..(height) {
            img.put_pixel(x, y, image::Luma([fax[((x - 1) * width + y - 1) as usize]]));
        }
    }
    img
}

// fn mainforpdf() -> Result<(), std::boxed::Box<dyn std::error::Error>> {
//     // Collect command-line arguments
//     let args: Vec<String> = env::args().collect();
//     if args.len() < 3 {
//         eprintln!(
//             "Usage: {} <template.json> <examkey.json> <scans.pdf>",
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

//     let file = pdf::file::FileOptions::cached().open(&args[3]).unwrap();
//     let resolver = file.resolver();

//     let mut scanned_docs: Vec<(u32, GrayImage)> = vec![];

//     for page_num in 0..3 {
//         let page = file.get_page(page_num + 1)?;

//         if let images = page
//             .resources()?
//             .xobjects
//             .iter()
//             .map(|(_name, &r)| resolver.get(r).unwrap())
//             .filter(|o| matches!(**o, XObject::Image(_)))
//         {
//             for (_i, o) in images.enumerate() {
//                 let img = match *o {
//                     XObject::Image(ref im) => im,
//                     _ => continue,
//                 };
//                 let (mut image_data, filter) = img.raw_image_data(&resolver).unwrap();
//                 println!("{:?}", filter);
//                 //                let format = match filter {
//                 //                     Some(pdf::enc::StreamFilter::DCTDecode(_)) => image::ImageFormat::Jpeg,
//                 //                     Some(pdf::enc::StreamFilter::FlateDecode(_)) => image::ImageFormat::Png,
//                 //                     Some(pdf::enc::StreamFilter::CCITTFaxDecode(_)) => {
//                 //                         image_data = fax::tiff::wrap(&image_data, img.width, img.height).into();
//                 //                         image::ImageFormat::Pnm
//                 //                     }
//                 //                     _ => continue,
//                 //                 };
//                 // //
//                 //            std::fs::write("test.tiff", &image_data)?;
//                 // let binary_image = binary_image_from_image(
//                 //     image::load_from_memory_with_format(&image_data, format).unwrap(),
//                 // );
//                 // scanned_docs.push((page_num, binary_image));
//             }
//         }
//     }

//     let res: Vec<String> = scanned_docs
//         .par_iter()
//         .map(|i| score_image(i.1.clone(), &t, &k, &format!("page number {}", i.0)))
//         .collect();

//     for s in res {
//         println!("{}", s);
//     }

//     Ok(())
// }

// fn main() -> Result<(), std::boxed::Box<dyn std::error::Error>> {
//     let image_path = std::path::Path::new("test.tiff");
//     let mut reader =
//         tiff::decoder::Decoder::new(File::open(image_path)?).expect("could not open image");
//     println!("{:?}", reader.colortype());
//     println!("{:?}", reader.chunk_dimensions());
//     println!("{:?}", reader.read_image());

//     //let img = image
//     Ok(())
// }
