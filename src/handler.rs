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
use percent_encoding::percent_decode_str;
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
                    let (bucket, key) = match build_bucket_and_object_key(uri, prefix, path) {
                        Ok((bucket, key)) => (bucket, key),
                        Err(err) => return Some(Err(err)),
                    };
                    return self.client.s3.get_object(bucket, key).await;
                }
                "http" | "https" => {
                    let url = build_url(uri, prefix, path);
                    return self.client.web.get(url).await;
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

fn build_bucket_and_object_key(
    src_uri: &Uri,
    req_prefix: &str,
    req_path: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let bucket = src_uri.host().ok_or("s3 client src is wrong")?;
    let decoded_path = percent_decode_str(req_path).decode_utf8()?;
    // /images
    let path_1 = src_uri.path();
    // foo/bar.jpg -> bar.jpg
    let path_2 = decoded_path
        .trim_start_matches(req_prefix)
        .trim_start_matches("/");
    // /images/bar.jpg
    if let Some(key_path) = Path::new(path_1).join(path_2).as_path().to_str() {
        // images/bar.jpg
        Ok((
            bucket.to_string(),
            key_path.trim_start_matches("/").to_string(),
        ))
    } else {
        Err(Box::from(
            "failed to build bucket and object key for s3 from request",
        ))
    }
}

fn build_url(src_uri: &Uri, req_prefix: &str, req_path: &str) -> String {
    let prefix = req_prefix.trim_start_matches("/").trim_end_matches("/");
    let path = req_path.trim_start_matches("/").trim_end_matches("/");
    format!(
        "{}/{}",
        src_uri.to_string().trim_end_matches("/"),
        path.trim_start_matches(prefix).trim_start_matches("/"),
    )
}

#[test]
fn test_build_bucket_and_object_key() {
    #[derive(Debug)]
    struct Case {
        src: &'static str,
        req_prefix: &'static str,
        req_path: &'static str,
        error: bool,
        want: (&'static str, &'static str),
    }
    let cases = [
        Case {
            src: "s3://local-test/images",
            req_prefix: "foo",
            req_path: "foo/dog.gif",
            error: false,
            want: ("local-test", "images/dog.gif"),
        },
        Case {
            src: "s3://local-test/images/",
            req_prefix: "/foo/",
            req_path: "/foo/dog.gif",
            error: false,
            want: ("local-test", "images/dog.gif"),
        },
        Case {
            src: "s3://local-test/images",
            req_prefix: "/foo",
            req_path: "/foo/dog.gif",
            error: false,
            want: ("local-test", "images/dog.gif"),
        },
        Case {
            src: "s3://local-test/images/",
            req_prefix: "foo/",
            req_path: "foo/dog.gif",
            error: false,
            want: ("local-test", "images/dog.gif"),
        },
        Case {
            src: "s3://local-test/images",
            req_prefix: "foo",
            req_path: "foo/犬.gif",
            error: false,
            want: ("local-test", "images/犬.gif"),
        },
        Case {
            src: "s3://local-test/images",
            req_prefix: "foo",
            req_path: "foo/%E7%8A%AC.gif",
            error: false,
            want: ("local-test", "images/犬.gif"),
        },
        Case {
            src: "s3://local-test/images/animals",
            req_prefix: "foo",
            req_path: "foo/bar/dog.gif",
            error: false,
            want: ("local-test", "images/animals/bar/dog.gif"),
        },
    ];
    for c in cases {
        let uri = c.src.parse::<Uri>().expect("case bug");
        match build_bucket_and_object_key(&uri, c.req_prefix, c.req_path) {
            Ok((got_bucket, got_key)) => {
                assert!(!c.error, "case: {c:?}");
                let (want_bucket, want_key) = c.want;
                assert_eq!(got_bucket, want_bucket);
                assert_eq!(got_key, want_key);
            }
            Err(err) => {
                assert!(c.error, "case: {c:?}, error: {err}");
            }
        }
    }
}

#[test]
fn test_buid_url() {
    #[derive(Debug)]
    struct Case {
        src: &'static str,
        req_prefix: &'static str,
        req_path: &'static str,
        want: &'static str,
    }
    let cases = [
        Case {
            src: "http://127.0.0.1/images",
            req_prefix: "foo",
            req_path: "foo/dog.gif",
            want: "http://127.0.0.1/images/dog.gif",
        },
        Case {
            src: "http://127.0.0.1/images/",
            req_prefix: "/foo/",
            req_path: "/foo/dog.gif",
            want: "http://127.0.0.1/images/dog.gif",
        },
        Case {
            src: "http://127.0.0.1/images",
            req_prefix: "/foo",
            req_path: "/foo/dog.gif",
            want: "http://127.0.0.1/images/dog.gif",
        },
        Case {
            src: "http://127.0.0.1/images/",
            req_prefix: "foo/",
            req_path: "foo/dog.gif",
            want: "http://127.0.0.1/images/dog.gif",
        },
        Case {
            src: "http://127.0.0.1/images",
            req_prefix: "foo",
            req_path: "foo/犬.gif",
            want: "http://127.0.0.1/images/犬.gif",
        },
        Case {
            src: "http://127.0.0.1/images",
            req_prefix: "foo",
            req_path: "foo/%E7%8A%AC.gif",
            want: "http://127.0.0.1/images/%E7%8A%AC.gif",
        },
        Case {
            src: "http://127.0.0.1/images/animals",
            req_prefix: "foo",
            req_path: "foo/bar/dog.gif",
            want: "http://127.0.0.1/images/animals/bar/dog.gif",
        },
    ];
    for c in cases {
        let uri = c.src.parse::<Uri>().expect("case bug");
        let got = build_url(&uri, c.req_prefix, c.req_path);
        assert_eq!(got, c.want, "case: {c:?}");
    }
}
