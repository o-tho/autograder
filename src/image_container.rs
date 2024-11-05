use crate::image_helpers::{binary_image_from_image, fax_to_grayimage};
use crate::ErrorWrapper;
use image::GrayImage;

use pdf::file::NoCache;
use pdf::file::NoLog;
use pdf::object::*;
use tiff::decoder::DecodingResult;
use tiff::TiffError;

pub struct PdfContainer {
    pub pdf_file: pdf::file::File<Vec<u8>, NoCache, NoCache, NoLog>,
}

pub struct TiffContainer<R: std::io::BufRead + std::io::Seek> {
    pub decoder: tiff::decoder::Decoder<R>,
}

pub trait ImageContainer {
    fn to_vector(&mut self) -> Result<Vec<GrayImage>, ErrorWrapper>;
}

impl<R: std::io::BufRead + std::io::Seek> ImageContainer for TiffContainer<R> {
    fn to_vector(&mut self) -> Result<Vec<GrayImage>, ErrorWrapper> {
        let mut result: Vec<GrayImage> = vec![];
        loop {
            let from_tiff = match self.decoder.read_image()? {
                DecodingResult::U8(buffer) => buffer,
                DecodingResult::U16(buffer) => buffer
                    .iter()
                    .flat_map(|&x| x.to_be_bytes().into_iter())
                    .collect::<Vec<u8>>(),
                _ => {
                    return Err(ErrorWrapper::TiffError(TiffError::UnsupportedError(
                        tiff::TiffUnsupportedError::UnsupportedDataType,
                    )))
                }
            };

            let (width, height) = self.decoder.dimensions()?;
            let gray = image::DynamicImage::ImageLuma8(
                GrayImage::from_raw(width, height, from_tiff).unwrap(),
            );
            result.push(binary_image_from_image(gray));
            if self.decoder.next_image().is_err() {
                break;
            }
        }
        Ok(result)
    }
}

impl ImageContainer for PdfContainer {
    fn to_vector(&mut self) -> Result<Vec<GrayImage>, ErrorWrapper> {
        let file = &self.pdf_file;
        let resolver = file.resolver();

        let mut scanned_docs: Vec<GrayImage> = vec![];

        for page_num in 0..file.num_pages() {
            let page = file.get_page(page_num)?;

            let images = page
                .resources()?
                .xobjects
                .iter()
                .map(|(_name, &r)| resolver.get(r).unwrap())
                .filter(|o| matches!(**o, XObject::Image(_)));

            for (_i, o) in images.enumerate() {
                let img = match *o {
                    XObject::Image(ref im) => im,
                    _ => continue,
                };
                let (image_data, filter) = img.raw_image_data(&resolver).unwrap();
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
                        fax_to_grayimage(&image_data, img.width, img.height)
                    }
                    _ => continue,
                };
                scanned_docs.push(grayimage);
            }
        }
        Ok(scanned_docs)
    }
}
