pub mod image_container;
pub mod image_helpers;
pub mod point;
pub mod report;
pub mod scan;
pub mod template;
pub mod typst_helpers;
pub mod webapp;

use crate::image_container::ImageContainer;
use crate::image_container::SingleImageContainer;
use crate::image_helpers::binary_image_from_image;

use crate::scan::Scan;
use template::{ExamKey, Template};

use itertools::Itertools;
use rayon::prelude::*;
pub fn generate_reports_for_image_container(
    container: &mut dyn ImageContainer,
    template: &Template,
    key: &ExamKey,
    out_prefix: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let iterator = container.to_iter();
    let mut all_records = Vec::new();
    let chunksize = 100;
    for (turn, chunk) in iterator.chunks(chunksize).into_iter().enumerate() {
        let images: Vec<image::GrayImage> = chunk.collect();

        // Process each chunk in parallel and collect the results
        let chunk_records: Vec<_> = images
            .par_iter()
            .enumerate()
            .map(|(idx, img)| {
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
                let file_name = report.save_filename(&"".to_string());
                (file_name, report.sid, report.score)
            })
            .collect();

        // Add this chunk's records to the main collection
        all_records.extend(chunk_records);
    }

    // Write all records to CSV
    let mut csv_writer = csv::Writer::from_writer(std::io::Cursor::new(Vec::new()));
    csv_writer.write_record(["Filename", "ID", "Score"])?;
    for record in all_records {
        csv_writer.serialize(record)?;
    }
    let csv_data = csv_writer.into_inner()?.into_inner();
    Ok(String::from_utf8(csv_data)?)
}

pub fn debug_report(container: &SingleImageContainer, template: &Template) {
    use crate::point::Point;
    let mut scan = Scan {
        img: binary_image_from_image(container.image.clone()),
        transformation: None,
    };
    scan.transformation = scan.find_transformation(template);
    let h_scale = (template.height as f64) / (scan.img.height() as f64);
    let w_scale = (template.width as f64) / (scan.img.width() as f64);

    let scale = (h_scale + w_scale) / 2.0;

    let projected_centers = template.circle_centers.map(|p| Point {
        x: (p.x as f64 / scale).round() as u32,
        y: (p.y as f64 / scale).round() as u32,
    });

    println!("expecting centers at {:#?}", projected_centers);

    scan.debug_report(template);
}
