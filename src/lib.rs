pub mod app;
pub mod image_container;
pub mod image_helpers;
pub mod point;
pub mod report;
pub mod scan;
pub mod template;

use crate::image_container::ImageContainer;
use crate::report::ImageReport;
use crate::scan::Scan;
use rayon::prelude::*;
use template::{ExamKey, Template};
use wasm_bindgen::prelude::*;

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
    let scanned_docs = container.to_vector()?;

    let res: Vec<ImageReport> = scanned_docs
        .par_iter()
        .enumerate()
        .map(|i| {
            let mut scan = Scan {
                img: i.1.clone(),
                transformation: None,
            };
            scan.transformation = scan.find_transformation(&template);
            scan.generate_imagereport(&template, &key, &format!("page{}", i.0))
        })
        .collect();

    for s in res {
        s.save_to_file(&out_prefix);
    }
    Ok(())
}
