use crate::image_helpers::{binary_image_from_image, create_error_image, fax_to_grayimage};
use image::{DynamicImage, GrayImage, ImageBuffer, ImageDecoder, Luma, Rgb, Rgba};

use pdf::any::AnySync;
use pdf::file::NoLog;
use pdf::file::SyncCache;
use pdf::object::*;
use std::sync::Arc;
use tiff::decoder::DecodingResult;

use std::iter;

pub struct PdfContainer {
    pub pdf_file: pdf::file::File<
        Vec<u8>,
        Arc<SyncCache<PlainRef, Result<AnySync, Arc<pdf::PdfError>>>>,
        Arc<SyncCache<PlainRef, Result<Arc<[u8]>, Arc<pdf::PdfError>>>>,
        NoLog,
    >,
}

pub struct TiffContainer<R: std::io::BufRead + std::io::Seek> {
    pub decoder: tiff::decoder::Decoder<R>,
}

pub struct SingleImageContainer {
    pub image: DynamicImage,
}

impl SingleImageContainer {
    pub fn from_data_with_format(data: &[u8], format: image::ImageFormat) -> Self {
        let reader = image::ImageReader::with_format(std::io::Cursor::new(data), format);
        if let Ok(mut decoder) = reader.into_decoder() {
            let orientation = decoder
                .orientation()
                .unwrap_or(image::metadata::Orientation::NoTransforms);

            if let Ok(mut dynimage) = image::DynamicImage::from_decoder(decoder) {
                dynimage.apply_orientation(orientation);

                return SingleImageContainer { image: dynimage };
            }
        }

        panic!("could not decode single image!");
    }
}

pub trait ImageContainer {
    fn get_page(&mut self, n: usize) -> Result<GrayImage, std::boxed::Box<dyn std::error::Error>>;

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
    fn get_page(&mut self, n: usize) -> Result<GrayImage, std::boxed::Box<dyn std::error::Error>> {
        if n == 0 {
            Ok(binary_image_from_image(self.image.clone()))
        } else {
            Err(std::boxed::Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Page out of range",
            )))
        }
    }
}

impl<R: std::io::BufRead + std::io::Seek> ImageContainer for TiffContainer<R> {
    fn get_page(&mut self, _n: usize) -> Result<GrayImage, std::boxed::Box<dyn std::error::Error>> {
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

            let color_type = match decoder.colortype() {
                Ok(t) => t,
                Err(_) => {
                    return Some(create_error_image(
                        "Could not determine color type. Consider scanning to grayscale",
                    ))
                }
            };

            let decoded = match color_type {
                tiff::ColorType::Gray(_) => {
                    ImageBuffer::<Luma<u8>, _>::from_raw(width, height, from_tiff)
                        .map(DynamicImage::ImageLuma8)
                }
                tiff::ColorType::RGB(_) => {
                    ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, from_tiff)
                        .map(DynamicImage::ImageRgb8)
                }
                tiff::ColorType::RGBA(_) => {
                    ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, from_tiff)
                        .map(DynamicImage::ImageRgba8)
                }
                _ => return Some(create_error_image(
                    "Cannot handle this color type. Please convert to PDF or scan to grayscale.",
                )),
            };

            // Convert the buffer into a GrayImage and process it
            let gray = decoded?.into_luma8().into();
            // Return the processed image
            Some(binary_image_from_image(gray))
        });

        Box::new(iter)
    }
}

impl ImageContainer for PdfContainer {
    fn get_page(&mut self, n: usize) -> Result<GrayImage, std::boxed::Box<dyn std::error::Error>> {
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
            match filter {
                Some(pdf::enc::StreamFilter::DCTDecode(_)) => {
                    match image::load_from_memory_with_format(&image_data, image::ImageFormat::Jpeg)
                    {
                        Ok(img) => Ok(binary_image_from_image(img)),
                        Err(e) => Ok(create_error_image(&format!(
                            "Failed to decode JPEG on page {}: {}",
                            n + 1,
                            e
                        ))),
                    }
                }
                Some(pdf::enc::StreamFilter::FlateDecode(_)) => {
                    match image::load_from_memory_with_format(&image_data, image::ImageFormat::Png)
                    {
                        Ok(img) => Ok(binary_image_from_image(img)),
                        Err(e) => Ok(create_error_image(&format!(
                            "Failed to decode PNG on page {}: {}",
                            n + 1,
                            e
                        ))),
                    }
                }
                Some(pdf::enc::StreamFilter::CCITTFaxDecode(_)) => {
                    Ok(fax_to_grayimage(&image_data, img.width, img.height))
                }
                _ => Ok(create_error_image(&format!(
                    "Could not decode image on page {}: Unsupported format",
                    n + 1
                ))),
            }
        } else {
            Ok(create_error_image(&format!("No image on page {}", n + 1)))
        }
    }
}
