use serde::Deserialize;

#[derive(Clone, Debug, Default, Eq, Deserialize)]
pub struct Query {
    w: Option<u32>,
    h: Option<u32>,
    rgb: Option<String>,
    quality: Option<u8>,
    crop: Option<bool>,
    avif: Option<bool>,
    webp: Option<bool>,
}

impl PartialEq for Query {
    fn eq(&self, rhs: &Self) -> bool {
        self.w == rhs.w
            && self.h == rhs.h
            && self.rgb == rhs.rgb
            && self.quality == rhs.quality
            && self.crop == rhs.crop
            && self.avif == rhs.avif
            && self.webp == rhs.webp
    }
}

impl Query {
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        match (self.w, self.h) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        }
    }

    pub fn fill_color(&self) -> (u8, u8, u8) {
        self.rgb.as_ref().map_or((32, 32, 32), |text| {
            let rgb: Vec<u8> = text
                .split(',')
                .take(3)
                .map(|e| e.parse::<u8>().map_or(32, |v| v))
                .collect();
            if rgb.len() != 3usize {
                return (32, 32, 32);
            }
            (rgb[0], rgb[1], rgb[2])
        })
    }

    pub fn quality(&self) -> u8 {
        self.quality.map_or(85, |v| v)
    }

    pub fn cropping(&self) -> bool {
        self.crop.is_some_and(|v| v)
    }

    pub fn use_avif(&self) -> bool {
        self.avif.is_some_and(|v| v)
    }

    pub fn use_webp(&self) -> bool {
        self.webp.is_some_and(|v| v)
    }

    pub fn as_is(&self) -> bool {
        self.dimensions().is_none() && !self.use_avif() && !self.use_webp()
    }

    pub fn unsupported_scale_size(&self) -> bool {
        let w = self.w.map_or(100, |v| v);
        let h = self.h.map_or(100, |v| v);
        !(20..=2000).contains(&w) || !(20..=1000).contains(&h)
    }
}

#[test]
fn test_query() {
    struct Case {
        query_string: &'static str,
        error: bool,
        want: Query,
        assert: fn(Query),
    }
    let cases = [
        Case {
            query_string: "http://127.0.0.1:3000",
            error: false,
            want: Query {
                ..Default::default()
            },
            assert: |got| {
                assert_eq!(got.dimensions(), None);
                assert_eq!(got.fill_color(), (32, 32, 32));
                assert_eq!(got.quality(), 85);
                assert!(!got.cropping());
                assert!(!got.use_avif());
                assert!(!got.use_webp());
                assert!(got.as_is());
                assert!(!got.unsupported_scale_size());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?w=",
            error: true,
            want: Query {
                ..Default::default()
            },
            assert: |_| {},
        },
        Case {
            query_string: "http://127.0.0.1:3000?unknown=1",
            error: false,
            want: Query {
                ..Default::default()
            },
            assert: |_| {},
        },
        Case {
            query_string: "http://127.0.0.1:3000?w=2000&h=1000",
            error: false,
            want: Query {
                w: Some(2000),
                h: Some(1000),
                ..Default::default()
            },
            assert: |got| {
                assert_eq!(got.dimensions(), Some((2000, 1000)));
                assert!(!got.as_is());
                assert!(!got.unsupported_scale_size());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?w=1618",
            error: false,
            want: Query {
                w: Some(1618),
                ..Default::default()
            },
            assert: |got| {
                assert_eq!(got.dimensions(), None);
                assert!(got.as_is());
                assert!(!got.unsupported_scale_size());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?w=2001&h=1001",
            error: false,
            want: Query {
                w: Some(2001),
                h: Some(1001),
                ..Default::default()
            },
            assert: |got| {
                assert_eq!(got.dimensions(), Some((2001, 1001)));
                assert!(!got.as_is());
                assert!(got.unsupported_scale_size());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?w=foo&h=bar",
            error: true,
            want: Query {
                ..Default::default()
            },
            assert: |_| {},
        },
        Case {
            query_string: "http://127.0.0.1:3000?rgb=255,255,255",
            error: false,
            want: Query {
                rgb: Some("255,255,255".to_string()),
                ..Default::default()
            },
            assert: |got| {
                assert_eq!(got.fill_color(), (255, 255, 255));
                assert!(got.as_is());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?rgb=255,255,255,255",
            error: false,
            want: Query {
                rgb: Some("255,255,255,255".to_string()),
                ..Default::default()
            },
            assert: |got| {
                assert_eq!(got.fill_color(), (255, 255, 255));
                assert!(got.as_is());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?rgb=255,255",
            error: false,
            want: Query {
                rgb: Some("255,255".to_string()),
                ..Default::default()
            },
            assert: |got| {
                assert_eq!(got.fill_color(), (32, 32, 32));
                assert!(got.as_is());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?rgb=foo,bar,baz",
            error: false,
            want: Query {
                rgb: Some("foo,bar,baz".to_string()),
                ..Default::default()
            },
            assert: |got| {
                assert_eq!(got.fill_color(), (32, 32, 32));
                assert!(got.as_is());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?quality=50",
            error: false,
            want: Query {
                quality: Some(50),
                ..Default::default()
            },
            assert: |got| {
                assert_eq!(got.quality(), 50);
                assert!(got.as_is());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?quality=foo",
            error: true,
            want: Query {
                ..Default::default()
            },
            assert: |_| {},
        },
        Case {
            query_string: "http://127.0.0.1:3000?crop=true",
            error: false,
            want: Query {
                crop: Some(true),
                ..Default::default()
            },
            assert: |got| {
                assert!(got.cropping());
                assert!(got.as_is());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?crop=foo",
            error: true,
            want: Query {
                ..Default::default()
            },
            assert: |_| {},
        },
        Case {
            query_string: "http://127.0.0.1:3000?avif=true",
            error: false,
            want: Query {
                avif: Some(true),
                ..Default::default()
            },
            assert: |got| {
                assert!(got.use_avif());
                assert!(!got.as_is());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?avif=foo",
            error: true,
            want: Query {
                ..Default::default()
            },
            assert: |_| {},
        },
        Case {
            query_string: "http://127.0.0.1:3000?webp=true",
            error: false,
            want: Query {
                webp: Some(true),
                ..Default::default()
            },
            assert: |got| {
                assert!(got.use_webp());
                assert!(!got.as_is());
            },
        },
        Case {
            query_string: "http://127.0.0.1:3000?webp=foo",
            error: true,
            want: Query {
                ..Default::default()
            },
            assert: |_| {},
        },
    ];
    for c in cases {
        let uri = c.query_string.parse::<axum::http::Uri>().unwrap();
        match axum::extract::Query::try_from_uri(&uri) {
            Ok(axum::extract::Query(got)) => {
                assert!(!c.error, "case: {}", c.query_string);
                assert!(
                    got == c.want,
                    "case: {}, want: {:?}, got: {:?}",
                    c.query_string,
                    c.want,
                    got,
                );
                (c.assert)(got);
            }
            Err(err) => {
                assert!(c.error, "case: {}, error: {err}", c.query_string);
            }
        }
    }
}
