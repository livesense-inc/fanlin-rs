use super::config;
use super::content;
use super::infra;
use super::query;
use image::{
    codecs::{avif, gif, jpeg, png},
    imageops::{overlay, FilterType},
    AnimationDecoder, DynamicImage, Frame, ImageBuffer, ImageDecoder, ImageFormat, ImageReader,
    Limits, Rgba, RgbaImage,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct State {
    router: matchit::Router<Provider>,
    client: infra::Client,
    fallback_images: HashMap<String, Vec<u8>>,
    fallback_path: String,
}

#[derive(Clone, Debug)]
struct Provider {
    path: String,
    src: axum::http::uri::Uri,
    fallback_path: String,
    success_even_no_content: bool,
}

impl State {
    pub fn new(providers: Vec<config::Provider>, client: infra::Client) -> Self {
        let router = Self::make_router(providers);
        let fallback_images: HashMap<String, Vec<u8>> = HashMap::new();
        let fallback_path = "".to_string();
        Self {
            router,
            client,
            fallback_images,
            fallback_path,
        }
    }

    fn make_router(providers: Vec<config::Provider>) -> matchit::Router<Provider> {
        let mut router = matchit::Router::new();
        for p in providers.iter() {
            let src = p
                .src
                .parse::<axum::http::uri::Uri>()
                .expect("failed to parse a provider src as URI");
            let path = p
                .path
                .trim_start_matches("/")
                .trim_end_matches("/")
                .to_string();
            let mut prefix = path.clone();
            if !prefix.is_empty() {
                prefix.insert(0, '/');
            }
            prefix.push_str("/{*p}");
            let fallback_path = p.fallback_path.clone().map_or("".to_string(), |v| v);
            let success_even_no_content = p.success_even_no_content.is_some_and(|v| v);
            let provider = Provider {
                path,
                src,
                fallback_path,
                success_even_no_content,
            };
            router
                .insert(prefix, provider)
                .expect("failed to make router with providers");
        }
        router
    }

    pub async fn with_fallback(
        &mut self,
        path: &Option<String>,
        providers: &[config::Provider],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(path) = path {
            if let Some(img) = self.get_image(path).await? {
                let _ = self.fallback_images.insert(path.clone(), img);
                self.fallback_path = path.clone();
            }
        }
        for provider in providers.iter() {
            if let Some(path) = &provider.fallback_path {
                if let Some(img) = self.get_image(path.as_str()).await? {
                    let _ = self.fallback_images.insert(path.clone(), img);
                }
            }
        }
        Ok(())
    }

    pub fn fallback(
        &self,
        req_path: &str,
        params: &query::Query,
        content: content::Format,
    ) -> Result<(&'static str, Vec<u8>), Box<dyn std::error::Error>> {
        match self.router.at(req_path) {
            Ok(matched) => {
                let provider = matched.value;
                match self.fallback_images.get(&provider.fallback_path) {
                    Some(img) => self.process_image(img, params, content),
                    None => match self.fallback_images.get(&self.fallback_path) {
                        Some(img) => self.process_image(img, params, content),
                        None => Err(Box::from("fallback image uninitialized")),
                    },
                }
            }
            Err(_) => match self.fallback_images.get(&self.fallback_path) {
                Some(img) => self.process_image(img, params, content),
                None => Err(Box::from("fallback image uninitialized")),
            },
        }
    }

    pub fn treat_as_success_even_no_content(&self, req_path: &str) -> bool {
        self.router
            .at(req_path)
            .is_ok_and(|v| v.value.success_even_no_content)
    }

    pub async fn get_image(
        &self,
        req_path: &str,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        // https://docs.rs/matchit/latest/matchit/index.html
        // https://docs.rs/matchit/latest/matchit/struct.Router.html
        match self.router.at(req_path) {
            Ok(matched) => {
                let provider = matched.value;
                let prefix = provider.path.as_str();
                let uri = &provider.src;
                match uri.scheme().map_or("", |v| v.as_str()) {
                    "s3" => {
                        let (bucket, key) = build_bucket_and_object_key(uri, prefix, req_path)?;
                        self.client.s3.get_object(bucket, key).await
                    }
                    "http" | "https" => {
                        let url = build_url(uri, prefix, req_path)?;
                        self.client.web.get(url).await
                    }
                    "file" => {
                        let local_path = build_local_path(uri, prefix, req_path)?;
                        self.client.file.read(local_path).await
                    }
                    _ => Ok(None),
                }
            }
            Err(_) => Ok(None),
        }
    }

    pub fn process_image(
        &self,
        original: &Vec<u8>,
        params: &query::Query,
        content: content::Format,
    ) -> Result<(&'static str, Vec<u8>), Box<dyn std::error::Error>> {
        // https://docs.rs/image/latest/image/struct.ImageReader.html
        let cursor = std::io::Cursor::new(original);
        let reader = ImageReader::new(cursor).with_guessed_format()?;
        let format = match reader.format() {
            Some(f) => match f {
                ImageFormat::Gif => f,
                _ if params.use_webp() && content.webp_accepted() => ImageFormat::WebP,
                _ if params.use_avif() && content.avif_accepted() => ImageFormat::Avif,
                _ => f,
            },
            None => return self.process_unknown_format(reader.into_inner().into_inner()),
        };
        if params.as_is() {
            return Ok((
                format.to_mime_type(),
                reader.into_inner().into_inner().to_owned(),
            ));
        }
        if format == ImageFormat::Gif {
            return self.process_gif(reader.into_inner().into_inner(), params);
        }
        let mut decoder = reader.into_decoder()?;
        // https://docs.rs/image/latest/image/enum.DynamicImage.html
        let mut img = match decoder.orientation() {
            Ok(o) => {
                let mut img = DynamicImage::from_decoder(decoder)?;
                img.apply_orientation(o);
                img
            }
            Err(_) => DynamicImage::from_decoder(decoder)?,
        };
        if params.grayscale() {
            img = img.grayscale();
        } else if params.inverse() {
            img.invert();
        }
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
        {
            let sigma = params.blur();
            if sigma > 0.0 {
                img = img.blur(sigma);
            }
        }
        let mut buffer = std::io::Cursor::new(Vec::new());
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
                let q = params.quality().clamp(1, 100);
                if q == 100 {
                    // https://docs.rs/image/latest/image/codecs/webp/struct.WebPEncoder.html
                    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buffer);
                    img.write_with_encoder(encoder)?;
                } else {
                    // https://docs.rs/webp/latest/webp/struct.Encoder.html
                    match webp::Encoder::from_image(&img) {
                        Ok(encoder) => {
                            buffer = std::io::Cursor::new(encoder.encode(q as f32).to_vec());
                        }
                        Err(_) => {
                            let e = image::codecs::webp::WebPEncoder::new_lossless(&mut buffer);
                            img.write_with_encoder(e)?;
                        }
                    };
                }
            }
            _ => img.write_to(&mut buffer, format)?,
        }
        Ok((format.to_mime_type(), buffer.into_inner()))
    }

    fn process_gif(
        &self,
        original: &Vec<u8>,
        params: &query::Query,
    ) -> Result<(&'static str, Vec<u8>), Box<dyn std::error::Error>> {
        let reader = std::io::Cursor::new(original);
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
                if params.grayscale() {
                    img = img.grayscale();
                } else if params.inverse() {
                    img.invert();
                }
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
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            // https://github.com/image-rs/image/issues/1983
            let mut encoder = gif::GifEncoder::new_with_speed(&mut buffer, 10);
            encoder.set_repeat(gif::Repeat::Infinite)?;
            encoder.encode_frames(frames.into_iter())?;
        }
        Ok((ImageFormat::Gif.to_mime_type(), buffer.into_inner()))
    }

    const MIME_TYPE_SVG: &'static str = "image/svg+xml";

    fn process_unknown_format(
        &self,
        original: &[u8],
    ) -> Result<(&'static str, Vec<u8>), Box<dyn std::error::Error>> {
        // https://docs.rs/resvg/latest/resvg/
        // https://docs.rs/usvg/latest/usvg/struct.Tree.html
        let _tree = {
            let opt = usvg::Options::default();
            usvg::Tree::from_data(original, &opt).map_err(|_err| "unknown format")?
        };
        Ok((Self::MIME_TYPE_SVG, original.to_owned()))
    }
}

fn build_bucket_and_object_key(
    src_uri: &axum::http::uri::Uri,
    req_prefix: &str,
    req_path: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let bucket = src_uri.host().ok_or("s3 client src is wrong")?;
    let path_1 = src_uri.path();
    let path_2 = clean_path(req_path, req_prefix)?;
    if let Some(key_path) = std::path::Path::new(path_1).join(path_2).as_path().to_str() {
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

const ASCII_SET: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
    .remove(b'.')
    .remove(b'/')
    .remove(b'-')
    .remove(b'_');

fn build_url(
    src_uri: &axum::http::uri::Uri,
    req_prefix: &str,
    req_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = src_uri.to_string();
    let path = clean_path(req_path, req_prefix)?;
    // https://docs.rs/percent-encoding/latest/percent_encoding/fn.utf8_percent_encode.html
    let encoded_path = percent_encoding::utf8_percent_encode(path.as_str(), ASCII_SET).to_string();
    let target_url = format!("{}/{}", url.trim_end_matches("/"), encoded_path);
    Ok(target_url)
}

fn build_local_path(
    src_uri: &axum::http::uri::Uri,
    req_prefix: &str,
    req_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // https://doc.rust-lang.org/std/path/struct.Path.html
    let path_1 = src_uri.path();
    let relative = path_1.starts_with("/./");
    let path_2 = clean_path(req_path, req_prefix)?;
    let local_path = std::path::Path::new(path_1)
        .join(path_2)
        .as_path()
        .to_str()
        .ok_or("failed to build local path from request")?
        .to_string();
    if relative {
        Ok(local_path.trim_start_matches("/./").to_string())
    } else {
        Ok(local_path)
    }
}

fn clean_path(raw_path: &str, prefix: &str) -> Result<String, Box<dyn std::error::Error>> {
    let decoded_path = percent_encoding::percent_decode_str(raw_path).decode_utf8()?;
    let mut target_path = decoded_path
        .trim_start_matches("/")
        .trim_start_matches(prefix.trim_start_matches("/").trim_end_matches("/"))
        .trim_start_matches("/")
        .to_string();
    loop {
        let tmp = target_path
            .replace("/../", "/")
            .replace("/./", "/")
            .replace("//", "/");
        let cleaned = target_path == tmp;
        target_path = tmp;
        if cleaned {
            break;
        }
    }
    target_path = target_path
        .trim_start_matches("../")
        .trim_start_matches("./")
        .to_string();
    Ok(target_path)
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
            src: "s3://local-test/images/",
            req_prefix: "foo",
            req_path: "/foo/dog.gif",
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
        let uri = c.src.parse::<axum::http::uri::Uri>().expect("case bug");
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
            src: "http://127.0.0.1/images/",
            req_prefix: "foo",
            req_path: "/foo/dog.gif",
            want: "http://127.0.0.1/images/dog.gif",
        },
        Case {
            src: "http://127.0.0.1/images",
            req_prefix: "foo",
            req_path: "foo/犬.gif",
            want: "http://127.0.0.1/images/%E7%8A%AC.gif",
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
        let uri = c.src.parse::<axum::http::uri::Uri>().expect("case bug");
        let got = build_url(&uri, c.req_prefix, c.req_path).expect("error");
        assert_eq!(got, c.want, "case: {c:?}");
    }
}

#[test]
fn test_buid_local_path() {
    #[derive(Debug)]
    struct Case {
        src: &'static str,
        req_prefix: &'static str,
        req_path: &'static str,
        error: bool,
        want: &'static str,
    }
    let cases = [
        Case {
            src: "file://locallhost/./images",
            req_prefix: "foo",
            req_path: "foo/dog.gif",
            error: false,
            want: "images/dog.gif",
        },
        Case {
            src: "file://locallhost/./images/",
            req_prefix: "/foo/",
            req_path: "/foo/dog.gif",
            error: false,
            want: "images/dog.gif",
        },
        Case {
            src: "file://locallhost/./images",
            req_prefix: "/foo",
            req_path: "/foo/dog.gif",
            error: false,
            want: "images/dog.gif",
        },
        Case {
            src: "file://locallhost/./images/",
            req_prefix: "foo/",
            req_path: "foo/dog.gif",
            error: false,
            want: "images/dog.gif",
        },
        Case {
            src: "file://locallhost/./images/",
            req_prefix: "foo",
            req_path: "/foo/dog.gif",
            error: false,
            want: "images/dog.gif",
        },
        Case {
            src: "file://locallhost/./images",
            req_prefix: "foo",
            req_path: "foo/犬.gif",
            error: false,
            want: "images/犬.gif",
        },
        Case {
            src: "file://locallhost/./images",
            req_prefix: "foo",
            req_path: "foo/%E7%8A%AC.gif",
            error: false,
            want: "images/犬.gif",
        },
        Case {
            src: "file://locallhost/./images/animals",
            req_prefix: "foo",
            req_path: "foo/bar/dog.gif",
            error: false,
            want: "images/animals/bar/dog.gif",
        },
        Case {
            src: "file://localhost/var/lib/images",
            req_prefix: "foo",
            req_path: "foo/dog.gif",
            error: false,
            want: "/var/lib/images/dog.gif",
        },
        Case {
            src: "file://localhost/var/lib/images",
            req_prefix: "foo",
            req_path: "foo/../../etc/passwd",
            error: false,
            want: "/var/lib/images/etc/passwd",
        },
        Case {
            src: "file://localhost/var/lib/images",
            req_prefix: "foo",
            req_path: "foo/.//....//..../etc/passwd",
            error: false,
            want: "/var/lib/images/..../..../etc/passwd",
        },
    ];
    for c in cases {
        let uri = c.src.parse::<axum::http::uri::Uri>().expect("case bug");
        match build_local_path(&uri, c.req_prefix, c.req_path) {
            Ok(got) => {
                assert!(!c.error, "case: {c:?}");
                assert_eq!(got, c.want, "case: {c:?}");
            }
            Err(err) => {
                assert!(c.error, "case: {c:?}, error: {err}");
            }
        }
    }
}
