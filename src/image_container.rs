use crate::image_helpers::{binary_image_from_image, fax_to_grayimage};
use crate::ErrorWrapper;
use image::{DynamicImage, GrayImage};

use pdf::file::NoCache;
use pdf::file::NoLog;
use pdf::object::*;
use tiff::decoder::DecodingResult;

use std::iter;

pub struct PdfContainer {
    pub pdf_file: pdf::file::File<Vec<u8>, NoCache, NoCache, NoLog>,
}

pub struct TiffContainer<R: std::io::BufRead + std::io::Seek> {
    pub decoder: tiff::decoder::Decoder<R>,
}

pub struct SingleImageContainer {
    pub image: DynamicImage,
}

pub trait ImageContainer {
    fn get_page(&mut self, n: usize) -> Result<GrayImage, ErrorWrapper>;

    fn to_iter(&mut self) -> Box<dyn Iterator<Item = GrayImage> + '_> {
        let mut page = 0;

        let iter = iter::from_fn(move || match self.get_page(page) {
            Ok(image) => {
                page += 1;
                Some(image)
            }
            Err(_) => None,
        });

        Box::new(iter)
    }

    fn to_vector(&mut self) -> Vec<GrayImage> {
        self.to_iter().collect()
    }
}

impl ImageContainer for SingleImageContainer {
    fn get_page(&mut self, n: usize) -> Result<GrayImage, ErrorWrapper> {
        if n == 0 {
            Ok(binary_image_from_image(self.image.clone()))
        } else {
            Err(ErrorWrapper::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Page out of range",
            )))
        }
    }
}

impl<R: std::io::BufRead + std::io::Seek> ImageContainer for TiffContainer<R> {
    fn get_page(&mut self, _n: usize) -> Result<GrayImage, ErrorWrapper> {
        unimplemented!();
    }
    fn to_iter(&mut self) -> Box<dyn Iterator<Item = GrayImage> + '_> {
        let decoder = &mut self.decoder;
        let mut first_call = true; // Flag to check if it's the first image

        let iter = std::iter::from_fn(move || {
            // Attempt to read the next image
            if !first_call && decoder.next_image().is_err() {
                return None; // End iteration if no more images are available
            }

            first_call = false;
            let from_tiff = match decoder.read_image() {
                Ok(DecodingResult::U8(buffer)) => buffer,
                Ok(DecodingResult::U16(buffer)) => buffer
                    .iter()
                    .flat_map(|&x| x.to_be_bytes().into_iter())
                    .collect::<Vec<u8>>(),
                Ok(_) => return None,  // Unsupported data type; end iteration
                Err(_) => return None, // Error reading; end iteration
            };

            // Get the dimensions of the image
            let (width, height) = match decoder.dimensions() {
                Ok(dimensions) => dimensions,
                Err(_) => return None, // Error getting dimensions; end iteration
            };

            // Convert the buffer into a GrayImage and process it
            let gray = image::DynamicImage::ImageLuma8(
                GrayImage::from_raw(width, height, from_tiff).unwrap(),
            );

            // Return the processed image
            Some(binary_image_from_image(gray))
        });

        Box::new(iter)
    }
}

impl ImageContainer for PdfContainer {
    fn get_page(&mut self, n: usize) -> Result<GrayImage, ErrorWrapper> {
        let file = &self.pdf_file;
        let resolver = file.resolver();

        let page = file.get_page(n as u32)?;

        let image = page.resources()?.xobjects.iter().find_map(|(_name, &r)| {
            resolver.get(r).ok().and_then(|o| match *o {
                XObject::Image(ref im) => Some(im.clone()),
                _ => None,
            })
        });

        if let Some(img) = image {
            let (image_data, filter) = img.raw_image_data(&resolver).unwrap();
            let grayimage: Result<GrayImage, ErrorWrapper> = match filter {
                Some(pdf::enc::StreamFilter::DCTDecode(_)) => Ok(binary_image_from_image(
                    image::load_from_memory_with_format(&image_data, image::ImageFormat::Jpeg)
                        .map_err(ErrorWrapper::ImageError)?,
                )),

                Some(pdf::enc::StreamFilter::FlateDecode(_)) => Ok(binary_image_from_image(
                    image::load_from_memory_with_format(&image_data, image::ImageFormat::Png)
                        .map_err(ErrorWrapper::ImageError)?,
                )),

                Some(pdf::enc::StreamFilter::CCITTFaxDecode(_)) => {
                    Ok(fax_to_grayimage(&image_data, img.width, img.height))
                }

                _ => Err(ErrorWrapper::ImageError(image::ImageError::Unsupported(
                    image::error::UnsupportedError::from_format_and_kind(
                        image::error::ImageFormatHint::Unknown,
                        image::error::UnsupportedErrorKind::GenericFeature(
                            "Unsupported feature".to_string(),
                        ),
                    ),
                ))),
            };
            grayimage
        } else {
            Err(ErrorWrapper::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Page has no image",
            )))
        }
    }
}
