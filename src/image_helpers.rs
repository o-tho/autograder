use crate::point::Point;
use fax::decoder;
use fax::Color;
use image::{DynamicImage, GrayImage, ImageReader, Luma, RgbImage};
use imageproc::drawing;
use std::path::Path;

fn kapur_level(img: &GrayImage) -> u8 {
    let hist = imageproc::stats::histogram(img);
    let histogram = &hist.channels[0]; // GrayImage has only one channel

    let total_pixels = (img.width() * img.height()) as f64;

    let mut cumulative_sum = [0.0f64; 257];
    let mut cumulative_entropy = [0.0f64; 257];

    for i in 0..256 {
        let p = histogram[i] as f64 / total_pixels;
        let entropy = if p > 0.0 { -p * p.ln() } else { 0.0 };
        cumulative_sum[i + 1] = cumulative_sum[i] + p;
        cumulative_entropy[i + 1] = cumulative_entropy[i] + entropy;
    }

    let mut max_entropy = f64::NEG_INFINITY;
    let mut optimal_threshold = 0;

    for threshold in 1..255 {
        let background_sum = cumulative_sum[threshold + 1];
        let foreground_sum = cumulative_sum[256] - background_sum;

        if background_sum < f64::EPSILON || foreground_sum < f64::EPSILON {
            continue;
        }

        let background_entropy = if background_sum > 0.0 {
            cumulative_entropy[threshold + 1] / background_sum
        } else {
            0.0
        };

        let foreground_entropy = if foreground_sum > 0.0 {
            (cumulative_entropy[256] - cumulative_entropy[threshold + 1]) / foreground_sum
        } else {
            0.0
        };

        let total_entropy = background_entropy + foreground_entropy;

        if total_entropy > max_entropy {
            max_entropy = total_entropy;
            optimal_threshold = threshold;
        }
    }

    optimal_threshold as u8
}

pub fn fax_to_grayimage(data: &[u8], width: u32, height: u32) -> GrayImage {
    let mut result = GrayImage::new(width, height);
    let mut y = 0;
    decoder::decode_g4(data.iter().cloned(), width as u16, None, |transitions| {
        for (x, c) in decoder::pels(transitions, width as u16).enumerate() {
            let pixel = match c {
                Color::Black => Luma([0u8]),
                Color::White => Luma([255u8]),
            };
            result.put_pixel(x as u32, y, pixel);
        }
        y += 1;
    });

    result
}
pub fn binary_image_from_image(img: DynamicImage) -> GrayImage {
    let res = img.into_luma8();
    let threshold = kapur_level(&res);

    imageproc::contrast::threshold(&res, threshold, imageproc::contrast::ThresholdType::Binary)
}

pub fn binary_image_from_file(path: &String) -> GrayImage {
    let image_path = Path::new(path);
    let img = ImageReader::open(image_path)
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
        drawing::draw_hollow_circle_mut(img, (center.x as i32, center.y as i32), radius + i, color);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn rgb_to_egui_color_image(image: &RgbImage) -> egui::ColorImage {
    let (width, height) = image.dimensions();
    let pixels: Vec<egui::Color32> = image
        .pixels()
        .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
        .collect();

    egui::ColorImage {
        size: [width as usize, height as usize],
        pixels,
    }
}
