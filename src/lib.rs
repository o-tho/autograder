pub mod image_container;
pub mod image_helpers;
pub mod point;
pub mod report;
pub mod scan;
pub mod template;
pub mod webapp;

use crate::image_container::ImageContainer;
use crate::scan::Scan;
use template::{ExamKey, Template};
use wasm_bindgen::prelude::*;

use itertools::Itertools;
use rayon::prelude::*;

// missing error types
use pdf::error::PdfError;
use serde_json::Error as SerdeError;
use tiff::TiffError;

#[derive(Debug)]
pub enum ErrorWrapper {
    TiffError(TiffError),
    PdfError(PdfError),
    JsonError(SerdeError),
    IoError(std::io::Error),
    ImageError(image::ImageError),
}

impl From<image::ImageError> for ErrorWrapper {
    fn from(error: image::ImageError) -> Self {
        ErrorWrapper::ImageError(error)
    }
}
impl From<TiffError> for ErrorWrapper {
    fn from(error: TiffError) -> Self {
        ErrorWrapper::TiffError(error)
    }
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
            ErrorWrapper::TiffError(e) => JsValue::from_str(&format!("Tiff Error: {:?}", e)),
            ErrorWrapper::IoError(e) => JsValue::from_str(&format!("I/O Error: {:?}", e)),
            ErrorWrapper::ImageError(e) => JsValue::from_str(&format!("Image Error: {:?}", e)),
        }
    }
}
pub fn generate_reports_for_image_container(
    container: &mut dyn ImageContainer,
    template: &Template,
    key: &ExamKey,
    out_prefix: String,
) -> Result<(), ErrorWrapper> {
    let iterator = container.to_iter();

    let mut turn = 0;
    let chunksize = 100;

    for chunk in &iterator.chunks(chunksize) {
        let images: Vec<image::GrayImage> = chunk.collect();

        images.par_iter().enumerate().for_each(|(idx, img)| {
            let mut scan = Scan {
                img: img.clone(),
                transformation: None,
            };
            scan.transformation = scan.find_transformation(template);
            let report = scan.generate_imagereport(
                template,
                key,
                &format!("page{}", idx + turn * chunksize),
            );
            report.save_to_file(&out_prefix);
        });
        turn += 1;
    }

    Ok(())
}
