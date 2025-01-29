use super::config;
use super::infra;
use super::query;
use axum::http::uri::Uri;
use image::{
    codecs::{jpeg, png, webp},
    imageops::{overlay, FilterType},
    DynamicImage, ImageBuffer, ImageFormat, ImageReader, Rgba,
};
use std::io::Cursor;

pub struct State {
    providers: Vec<config::Provider>,
    client: infra::Client,
}

impl State {
    pub fn new(providers: Vec<config::Provider>, client: infra::Client) -> Self {
        Self { providers, client }
    }

    pub async fn get_image(
        &self,
        orig_path: &str,
    ) -> Option<Result<Vec<u8>, Box<dyn std::error::Error>>> {
        let path = orig_path.trim_start_matches("/");
        if path.len() == 0 {
            return None;
        }
        for provider in self.providers.iter() {
            let prefix = provider.path.trim_start_matches("/");
            if !path.starts_with(prefix) {
                continue;
            }
            let uri = &provider.src.parse::<Uri>().unwrap();
            match uri.scheme().unwrap().as_str() {
                "s3" => {
                    let bucket = uri.host().unwrap();
                    let key = path.trim_start_matches(prefix);
                    return self.client.s3.get_object(bucket, key).await;
                }
                "http" | "https" => {
                    let url = format!("{}{}", provider.src, path.trim_start_matches(prefix));
                    return self.client.web.get_image(url).await;
                }
                _ => return None,
            }
        }
        return None;
    }

    pub fn process_image(
        &self,
        original: Vec<u8>,
        params: query::Query,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // https://docs.rs/image/latest/image/struct.ImageReader.html
        let cursor = Cursor::new(original);
        let reader = ImageReader::new(cursor).with_guessed_format()?;
        let format: image::ImageFormat;
        if params.use_webp() {
            format = ImageFormat::WebP;
        } else {
            format = match reader.format() {
                Some(f) => f,
                None => return Err(Box::from("unknown format")),
            };
        }
        // https://docs.rs/image/latest/image/enum.DynamicImage.html
        let mut img = reader.decode()?;
        if let Some((width, height)) = params.dimensions() {
            // https://docs.rs/image/latest/image/struct.ImageBuffer.html
            if width != img.width() || height != img.height() {
                if params.cropping() {
                    img = img.resize_to_fill(width, height, FilterType::Lanczos3);
                } else {
                    img = img.resize(width, height, FilterType::Lanczos3);
                }
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
            ImageFormat::Jpeg => {
                let q = match params.quality() {
                    n if n < 1 => 1,
                    n if n > 100 => 100,
                    n => n,
                };
                let mut encoder = jpeg::JpegEncoder::new_with_quality(&mut buffer, q);
                encoder.encode_image(&img)?;
            }
            ImageFormat::WebP => {
                // https://docs.rs/image/latest/image/codecs/webp/struct.WebPEncoder.html
                match params.quality() {
                    100 => {
                        let encoder = webp::WebPEncoder::new_lossless(&mut buffer);
                        img.write_with_encoder(encoder)?;
                    }
                    _ => {
                        // TODO: support lossy encoding
                        let encoder = webp::WebPEncoder::new_lossless(&mut buffer);
                        img.write_with_encoder(encoder)?;
                    }
                };
            }
            _ => img.write_to(&mut buffer, format)?,
        }
        Ok(buffer.into_inner())
    }
}
