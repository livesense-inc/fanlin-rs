use super::config;
use super::infra;
use super::query;
use image::{
    codecs::png,
    imageops::{overlay, FilterType},
    DynamicImage, ImageBuffer, ImageFormat, ImageReader, Rgba,
};
use std::io::Cursor;

pub struct State {
    config: config::Config,
    client: infra::Client,
    pub root_uri: String,
}

impl State {
    pub fn new(config: config::Config, client: infra::Client) -> Self {
        let root_uri = format!("http://127.0.0.1:{}", config.port);
        Self {
            config,
            client,
            root_uri,
        }
    }

    pub async fn get_image(
        &self,
        path: &str,
    ) -> Option<Result<Vec<u8>, Box<dyn std::error::Error>>> {
        let bucket = self.config.aws_s3_bucket;
        let key = path.trim_start_matches("/");
        if key.len() == 0 {
            return None;
        }
        self.client.get_s3_object(&bucket, &key).await
    }

    pub fn process_image(
        &self,
        original: Vec<u8>,
        params: query::Query,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // https://docs.rs/image/latest/image/struct.ImageReader.html
        let cursor = Cursor::new(original);
        let reader = ImageReader::new(cursor).with_guessed_format()?;
        let format = match reader.format() {
            Some(f) => f,
            None => return Err(Box::from("unknown format")),
        };
        // https://docs.rs/image/latest/image/enum.DynamicImage.html
        let mut img = reader.decode()?;
        if let Some((width, height)) = params.dimensions() {
            // https://docs.rs/image/latest/image/struct.ImageBuffer.html
            if width != img.width() || height != img.height() {
                img = img.resize(width, height, FilterType::Lanczos3);
            }
            if width > img.width() || height > img.height() {
                let (r, g, b) = params.fill_color();
                let mut bg = ImageBuffer::from_pixel(width, height, Rgba([r, g, b, 255]));
                overlay(
                    &mut bg,
                    &img,
                    (width.abs_diff(img.width()) / 2) as i64,
                    (height.abs_diff(img.height()) / 2) as i64,
                );
                img = DynamicImage::ImageRgba8(bg);
            }
        }
        let mut buffer = Cursor::new(Vec::new());
        match format {
            // https://docs.rs/image/latest/image/codecs/index.html
            ImageFormat::Png => {
                let ct = match params.quality() {
                    n if n < 50 => png::CompressionType::Best,
                    n if n < 85 => png::CompressionType::Default,
                    _ => png::CompressionType::Fast,
                };
                let ft = png::FilterType::Adaptive;
                let encoder = png::PngEncoder::new_with_quality(&mut buffer, ct, ft);
                img.write_with_encoder(encoder)?;
            }
            _ => img.write_to(&mut buffer, format)?,
        }
        Ok(buffer.into_inner())
    }
}
