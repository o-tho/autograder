use image::RgbImage;

pub struct ImageReport {
    pub image: RgbImage,
    pub sid: Option<u32>,
    pub version: Option<u32>,
    pub score: u32,
    pub identifier: String,
}

impl ImageReport {
    pub fn save_to_file(&self, prefix: &String) {
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

        let _ = self
            .image
            .save_with_format(prefix.to_string() + &filename, image::ImageFormat::Png);
    }
}
