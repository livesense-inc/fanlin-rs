use super::config;
use super::infra;
use super::query;
use axum::http::uri::Uri;
use image::{
    codecs::{avif, gif, jpeg, png, webp},
    imageops::{overlay, FilterType},
    DynamicImage, Frame, ImageBuffer, ImageFormat, ImageReader, Limits, Rgba, RgbaImage,
};
use image::{AnimationDecoder, ImageDecoder};
use std::{io::Cursor, path::Path};

#[derive(Clone, Debug)]
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
        // /foo/bar.jpg -> foo/bar.jpg
        let path = orig_path.trim_start_matches("/");
        if path.is_empty() {
            return None;
        }
        for provider in self.providers.iter() {
            // /foo -> foo
            let prefix = provider.path.trim_start_matches("/");
            if !path.starts_with(prefix) {
                continue;
            }
            let uri = &provider.src.parse::<Uri>().unwrap();
            match uri.scheme().unwrap().as_str() {
                "s3" => {
                    let bucket = uri.host().unwrap();
                    // /images
                    let path_1 = uri.path();
                    // foo/bar.jpg -> bar.jpg
                    let path_2 = path.trim_start_matches(prefix).trim_start_matches("/");
                    // /images/bar.jpg
                    if let Some(key_path) = Path::new(path_1).join(path_2).as_path().to_str() {
                        // images/bar.jpg
                        let key = key_path.trim_start_matches("/");
                        return self.client.s3.get_object(bucket, key).await;
                    } else {
                        return Some(Err(Box::from("wrong s3 setting")));
                    }
                }
                "http" | "https" => {
                    let url = format!(
                        "{}{}",
                        provider.src.trim_end_matches("/"),
                        path.trim_start_matches(prefix)
                    );
                    return self.client.web.get_image(url).await;
                }
                _ => return None,
            }
        }
        None
    }

    pub fn process_image(
        &self,
        original: Vec<u8>,
        params: query::Query,
    ) -> Result<(&'static str, Vec<u8>), Box<dyn std::error::Error>> {
        // https://docs.rs/image/latest/image/struct.ImageReader.html
        let cursor = Cursor::new(original);
        let reader = ImageReader::new(cursor).with_guessed_format()?;
        let format: image::ImageFormat = if params.use_avif() {
            ImageFormat::Avif
        } else if params.use_webp() {
            ImageFormat::WebP
        } else {
            match reader.format() {
                Some(f) => f,
                None => return Err(Box::from("unknown format")),
            }
        };
        if params.as_is() {
            return Ok((format.to_mime_type(), reader.into_inner().into_inner()));
        }
        if format == ImageFormat::Gif {
            return self.process_gif(reader.into_inner().into_inner(), params);
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
                let q = params.quality().clamp(1, 100);
                let mut encoder = jpeg::JpegEncoder::new_with_quality(&mut buffer, q);
                encoder.encode_image(&img)?;
            }
            ImageFormat::Avif => {
                // https://docs.rs/image/latest/image/codecs/avif/struct.AvifEncoder.html
                let q = params.quality().clamp(1, 100);
                let encoder = avif::AvifEncoder::new_with_speed_quality(&mut buffer, 10, q)
                    .with_colorspace(avif::ColorSpace::Srgb);
                img.write_with_encoder(encoder)?;
            }
            ImageFormat::WebP => {
                // https://docs.rs/image/latest/image/codecs/webp/struct.WebPEncoder.html
                // TODO: support lossy encoding
                let encoder = webp::WebPEncoder::new_lossless(&mut buffer);
                img.write_with_encoder(encoder)?;
            }
            _ => img.write_to(&mut buffer, format)?,
        }
        Ok((format.to_mime_type(), buffer.into_inner()))
    }

    fn process_gif(
        &self,
        original: Vec<u8>,
        params: query::Query,
    ) -> Result<(&'static str, Vec<u8>), Box<dyn std::error::Error>> {
        let reader = Cursor::new(original);
        // https://docs.rs/image/latest/image/codecs/gif/index.html
        let mut decoder = gif::GifDecoder::new(reader)?;
        decoder.set_limits(Limits::no_limits())?;
        // https://docs.rs/image/latest/image/struct.Frames.html
        let frames: Vec<_> = decoder
            .into_frames()
            .map(|result| {
                // https://docs.rs/image/latest/image/struct.Frame.html
                if result.is_err() {
                    return Frame::new(RgbaImage::from_pixel(1, 1, Rgba([32, 32, 32, 255])));
                }
                let mut img = DynamicImage::ImageRgba8(result.unwrap().into_buffer());
                if let Some((width, height)) = params.dimensions() {
                    // https://docs.rs/image/latest/image/enum.DynamicImage.html
                    if width != img.width() || height != img.height() {
                        if params.cropping() {
                            img = img.resize_to_fill(width, height, FilterType::Nearest);
                        } else {
                            img = img.resize(width, height, FilterType::Nearest);
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
                Frame::new(img.to_rgba8())
            })
            .collect();
        let mut buffer = Cursor::new(Vec::new());
        {
            // https://github.com/image-rs/image/issues/1983
            let mut encoder = gif::GifEncoder::new_with_speed(&mut buffer, 10);
            encoder.set_repeat(gif::Repeat::Infinite)?;
            encoder.encode_frames(frames.into_iter())?;
        }
        Ok((ImageFormat::Gif.to_mime_type(), buffer.into_inner()))
    }
}
