use crate::point::Point;
use fax::decoder;
use fax::Color;
use image::{DynamicImage, GrayImage, ImageReader, Luma, RgbImage};
use imageproc::drawing;
use std::path::Path;

pub fn fax_to_grayimage(data: &[u8], width: u32, height: u32) -> GrayImage {
    let mut result = GrayImage::new(width, height);
    let mut y = 0;
    decoder::decode_g4(data.iter().cloned(), width as u16, None, |transitions| {
        let mut x = 0;
        for c in decoder::pels(transitions, width as u16) {
            let pixel = match c {
                Color::Black => Luma([0u8]),
                Color::White => Luma([255u8]),
            };
            result.put_pixel(x, y, pixel);
            x += 1;
        }
        y += 1;
    });

    result
}
pub fn binary_image_from_image(img: DynamicImage) -> GrayImage {
    let res = img.into_luma8();
    let threshold = imageproc::contrast::otsu_level(&res);

    imageproc::contrast::threshold(&res, threshold, imageproc::contrast::ThresholdType::Binary)
}

pub fn binary_image_from_file(path: &String) -> GrayImage {
    let image_path = Path::new(path);
    let img = ImageReader::open(&image_path)
        .expect("failed to open file")
        .decode()
        .expect("failed to decode image");

    binary_image_from_image(img)
}
pub fn gray_to_rgb(gray_image: &GrayImage) -> RgbImage {
    let (width, height) = gray_image.dimensions();
    let mut rgb_image = RgbImage::new(width, height);

    for (x, y, gray_pixel) in gray_image.enumerate_pixels() {
        let intensity = gray_pixel[0];
        rgb_image.put_pixel(x, y, image::Rgb([intensity, intensity, intensity]));
    }

    rgb_image
}

pub fn draw_circle_around_box(
    img: &mut RgbImage,
    topleft: Point,
    botright: Point,
    color: image::Rgb<u8>,
) {
    let radius = ((botright.x - topleft.x) / 3) as i32;
    let center = Point {
        x: (topleft.x + botright.x) / 2,
        y: (topleft.y + botright.y) / 2,
    };

    for i in 0..(radius / 4) {
        drawing::draw_hollow_circle_mut(
            img,
            (center.x as i32, center.y as i32),
            radius + i as i32,
            color,
        );
    }
}
