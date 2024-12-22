use crate::point::Point;
use fax::decoder;
use fax::Color;
use image::{DynamicImage, GrayImage, ImageReader, Luma, Rgb, RgbImage};
use imageproc::drawing;

use std::collections::HashMap;
use std::path::Path;
/// Returns the [Kapur threshold level] of an 8bpp image. This threshold
/// maximizes the entropy of the background and foreground.
///
/// [Kapur threshold level]: https://doi.org/10.1016/0734-189X(85)90125-2
pub fn kapur_level(img: &GrayImage) -> u8 {
    // The implementation looks different to the one you can for example find in
    // ImageMagick, because we are using the simplification of equation (18) in
    // the original article, which allows the computation of the total entropy
    // without having to use nested loops. The names of the variables are taken
    // straight from the article.
    let hist = imageproc::stats::histogram(img);
    let histogram = &hist.channels[0];
    const N: usize = 256;

    let total_pixels = (img.width() * img.height()) as f64;

    // The p_i in the article. They describe the probability of encountering
    // gray-level i.
    let mut p = [0.0f64; N];
    for i in 0..N {
        p[i] = histogram[i] as f64 / total_pixels;
    }

    // The P_s in the article, which is the probability of encountering
    // gray-level <= s.
    let mut cum_p = [0.0f64; N];
    cum_p[0] = p[0];
    for i in 1..N {
        cum_p[i] = cum_p[i - 1] + p[i];
    }

    // The H_s in the article. These are the entropies attached to the
    // distributions p[0],...,p[s].
    let mut h = [0.0f64; N];
    if p[0] > 0.0 {
        h[0] = -p[0] * p[0].ln();
    }
    for s in 1..N {
        h[s] = if p[s] > 0.0 {
            h[s - 1] - p[s] * p[s].ln()
        } else {
            h[s - 1]
        };
    }

    let mut max_entropy = f64::MIN;
    let mut best_threshold = 0;

    for s in 0..N {
        let pq = cum_p[s] * (1.0 - cum_p[s]);
        if pq <= 0.0 {
            continue;
        }

        // psi_s is the sum of the total entropy of foreground and
        // background at threshold level s. Instead of computing them
        // separately, we use equation (18) of the original article, which
        // simplifies it to this:
        let psi_s = pq.ln() + h[s] / cum_p[s] + (h[255] - h[s]) / (1.0 - cum_p[s]);
        if psi_s > max_entropy {
            max_entropy = psi_s;
            best_threshold = s;
        }
    }

    best_threshold as u8
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

    // we don't trust binary images and erode them first
    imageproc::morphology::erode_mut(&mut result, imageproc::distance_transform::Norm::L1, 1);
    result
}
pub fn binary_image_from_image(img: DynamicImage) -> GrayImage {
    let gray = img.into_luma8();
    let threshold = kapur_level(&gray);

    imageproc::contrast::threshold(&gray, threshold, imageproc::contrast::ThresholdType::Binary)
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
    let gray_data = gray_image.as_raw();
    let mut rgb_data = Vec::with_capacity(gray_data.len() * 3);

    for &intensity in gray_data {
        rgb_data.extend_from_slice(&[intensity, intensity, intensity]);
    }

    image::ImageBuffer::from_raw(width, height, rgb_data).unwrap()
}

pub fn replace_colour(
    image: &mut RgbImage,
    x_min: u32,
    y_min: u32,
    x_max: u32,
    y_max: u32,
    from: Rgb<u8>,
    to: Rgb<u8>,
) {
    let mut replacements = HashMap::new();
    replacements.insert(from, to);
    replace_colours(image, x_min, y_min, x_max, y_max, replacements);
}

pub fn replace_colours(
    image: &mut RgbImage,
    x_min: u32,
    y_min: u32,
    x_max: u32,
    y_max: u32,
    replacements: HashMap<Rgb<u8>, Rgb<u8>>,
) {
    let width = image.width();
    let height = image.height();

    let x_min = x_min.min(width - 1);
    let y_min = y_min.min(height - 1);
    let x_max = x_max.min(width - 1);
    let y_max = y_max.min(height - 1);

    for y in y_min..=y_max {
        for x in x_min..=x_max {
            let pixel = image.get_pixel_mut(x, y);
            if let Some(&replacement) = replacements.get(pixel) {
                *pixel = replacement;
            }
        }
    }
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

pub fn create_error_image(error_text: &str) -> GrayImage {
    let mut image = GrayImage::new(1200, 300);

    for pixel in image.pixels_mut() {
        *pixel = image::Luma([255u8]);
    }

    let font_data = crate::typst_helpers::BIOLINUM_BOLD;
    let font = ab_glyph::FontArc::try_from_slice(font_data).expect("Error loading font");

    let scale = ab_glyph::PxScale::from(30.0);

    let x = 20;
    let y = 150;

    imageproc::drawing::draw_text_mut(
        &mut image,
        image::Luma([0u8]),
        x,
        y,
        scale,
        &font,
        error_text,
    );

    image
}
