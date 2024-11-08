use image::RgbImage;
use std::io::{Cursor, Write};
use zip::write::FileOptions;
use zip::ZipWriter;

pub struct ImageReport {
    pub image: RgbImage,
    pub sid: Option<u32>,
    pub version: Option<u32>,
    pub score: u32,
    pub identifier: String,
}

impl ImageReport {
    pub fn save_filename(&self, prefix: &String) -> String {
        let mut filename: String = "".to_string();
        if let Some(id) = self.sid {
            filename += &format!("{}-", id);
        } else {
            filename += &format!("NOID-");
        }

        if let Some(version) = self.version {
            filename += &format!("v{}-", version);
        } else {
            filename += &format!("NOVERSION-");
        }

        filename += &format!("score{}-{}.png", self.score, self.identifier);

        prefix.to_string() + &filename
    }
    pub fn save_to_file(&self, prefix: &String) {
        let path = self.save_filename(&prefix);
        let _ = self.image.save_with_format(&path, image::ImageFormat::Png);
    }
    pub fn write_to_buffer(&self, buffer: &mut Vec<u8>) {
        let _ = self
            .image
            .write_to(&mut std::io::Cursor::new(buffer), image::ImageFormat::Png);
    }
}

pub fn create_zip_from_imagereports(
    reports: &Vec<ImageReport>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Create a buffer to hold the zip file in memory
    let mut zip_buffer = Cursor::new(Vec::new());
    let mut zip_writer = ZipWriter::new(&mut zip_buffer);
    let mut wrt = csv::Writer::from_writer(std::io::Cursor::new(Vec::new()));

    wrt.write_record(&["Filename", "ID", "Score"])?;
    for (_index, report) in reports.iter().enumerate() {
        // Encode each image as PNG into a separate buffer
        let mut image_buffer = Vec::new();
        report.write_to_buffer(&mut image_buffer);

        // Define a filename for each image within the zip
        let file_name = report.save_filename(&"".to_string());

        wrt.serialize((&file_name, report.sid, report.score))?;

        // Add the encoded image to the zip archive
        zip_writer.start_file::<String, ()>(
            file_name,
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )?;
        zip_writer.write_all(&image_buffer)?;
    }

    let csvdata = wrt.into_inner()?.into_inner();
    zip_writer.start_file::<String, ()>(
        "results.csv".to_string(),
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated),
    )?;
    zip_writer.write_all(&csvdata)?;

    // Finalize the zip archive
    zip_writer.finish()?;

    // Extract the resulting zip data from the buffer
    Ok(zip_buffer.into_inner())
}
