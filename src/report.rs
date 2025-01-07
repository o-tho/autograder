use image::RgbImage;
use std::io::Write;
use zip::write::FileOptions;
use zip::ZipWriter;

pub struct ImageReport {
    pub image: RgbImage,
    pub sid: Option<u32>,
    pub version: Option<u32>,
    pub issue: bool,
    pub score: u32,
    pub identifier: String,
}

impl ImageReport {
    pub fn save_filename(&self, prefix: &String) -> String {
        let mut filename: String = "".to_string();

        if self.issue {
            filename += "GRADE_BY_HAND-";
        }
        if let Some(id) = self.sid {
            filename += &format!("{}-", id);
        } else {
            filename += "NOID-";
        }

        if let Some(version) = self.version {
            filename += &format!("v{}-", version);
        } else {
            filename += "NOVERSION-";
        }

        filename += &format!("score{}-{}.png", self.score, self.identifier);

        prefix.to_string() + &filename
    }
    pub fn save_to_file(&self, prefix: &String) {
        let path = self.save_filename(prefix);
        let _ = self.image.save_with_format(&path, image::ImageFormat::Png);
    }
    pub fn write_to_buffer(&self, buffer: &mut Vec<u8>) {
        let _ = self
            .image
            .write_to(&mut std::io::Cursor::new(buffer), image::ImageFormat::Png);
    }

    pub fn add_to_zip<W: Write + std::io::Seek>(
        &self,
        zip_writer: &mut ZipWriter<W>,
        csv_writer: &mut csv::Writer<std::io::Cursor<Vec<u8>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Encode the image as PNG into a separate buffer
        let mut image_buffer = Vec::new();
        self.write_to_buffer(&mut image_buffer);

        // Define a filename for each image within the zip
        let file_name = self.save_filename(&"".to_string());

        // Serialize the metadata into the CSV writer
        csv_writer.serialize((&file_name, self.sid, self.score))?;

        // Add the encoded image to the zip archive
        zip_writer.start_file::<String, ()>(
            file_name,
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )?;
        zip_writer.write_all(&image_buffer)?;

        Ok(())
    }
}
