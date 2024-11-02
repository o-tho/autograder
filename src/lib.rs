mod image_helpers;
pub mod point;
pub mod report;
pub mod scan;
pub mod template;

use crate::report::ImageReport;
use crate::scan::{binary_image_from_image, Scan};
use image::GrayImage;
use pdf::object::*;
use rayon::prelude::*;
use std::fs::File;
use template::{ExamKey, Template};
use wasm_bindgen::prelude::*;

// missing error types

use pdf::error::PdfError;
use serde_json::Error as SerdeError;

#[derive(Debug)]
pub enum ErrorWrapper {
    PdfError(PdfError),
    JsonError(SerdeError),
    IoError(std::io::Error),
}

impl From<PdfError> for ErrorWrapper {
    fn from(error: PdfError) -> Self {
        ErrorWrapper::PdfError(error)
    }
}

impl From<SerdeError> for ErrorWrapper {
    fn from(error: SerdeError) -> Self {
        ErrorWrapper::JsonError(error)
    }
}

impl From<std::io::Error> for ErrorWrapper {
    fn from(error: std::io::Error) -> Self {
        ErrorWrapper::IoError(error)
    }
}

// Implement conversion from MyError to JsValue for Wasm compatibility
impl From<ErrorWrapper> for JsValue {
    fn from(error: ErrorWrapper) -> Self {
        match error {
            ErrorWrapper::PdfError(e) => JsValue::from_str(&format!("PDF Error: {:?}", e)),
            ErrorWrapper::JsonError(e) => JsValue::from_str(&format!("JSON Error: {:?}", e)),
            ErrorWrapper::IoError(e) => JsValue::from_str(&format!("I/O Error: {:?}", e)),
        }
    }
}
#[wasm_bindgen]
pub fn generate_reports_for_pdf(
    pdf_path: String,
    template_path: String,
    exam_key_path: String,
    out_prefix: String,
) -> Result<(), ErrorWrapper> {
    let t: Template = serde_json::from_reader(File::open(template_path)?)?;
    let k: ExamKey = serde_json::from_reader(File::open(exam_key_path)?)?;

    let file = pdf::file::FileOptions::cached().open(pdf_path).unwrap();
    let resolver = file.resolver();

    let mut scanned_docs: Vec<(u32, GrayImage)> = vec![];

    for page_num in 0..file.num_pages() {
        let page = file.get_page(page_num)?;

        if let images = page
            .resources()?
            .xobjects
            .iter()
            .map(|(_name, &r)| resolver.get(r).unwrap())
            .filter(|o| matches!(**o, XObject::Image(_)))
        {
            for (_i, o) in images.enumerate() {
                let img = match *o {
                    XObject::Image(ref im) => im,
                    _ => continue,
                };
                let (mut image_data, filter) = img.raw_image_data(&resolver).unwrap();
                let grayimage = match filter {
                    Some(pdf::enc::StreamFilter::DCTDecode(_)) => binary_image_from_image(
                        image::load_from_memory_with_format(&image_data, image::ImageFormat::Jpeg)
                            .unwrap(),
                    ),

                    Some(pdf::enc::StreamFilter::FlateDecode(_)) => binary_image_from_image(
                        image::load_from_memory_with_format(&image_data, image::ImageFormat::Png)
                            .unwrap(),
                    ),

                    Some(pdf::enc::StreamFilter::CCITTFaxDecode(_)) => {
                        image_helpers::fax_to_grayimage(&image_data, img.width, img.height)
                    }
                    _ => continue,
                };
                //grayimage.save_with_format("out.png", image::ImageFormat::Png);
                scanned_docs.push((page_num + 1, grayimage));
            }
        }
    }

    let res: Vec<ImageReport> = scanned_docs
        .par_iter()
        .map(|i| {
            let mut scan = Scan {
                img: i.1.clone(),
                transformation: None,
            };
            scan.transformation = scan.find_transformation(&t);
            scan.generate_imagereport(&t, &k, &format!("page{}", i.0))
        })
        .collect();

    for s in res {
        s.save_to_file(&out_prefix);
    }
    Ok(())
}
