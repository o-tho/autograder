use fax::decoder;
use fax::Color;
use image::{GrayImage, Luma};

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
