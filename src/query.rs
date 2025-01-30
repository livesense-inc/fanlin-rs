use serde::Deserialize;

#[derive(Deserialize)]
pub struct Query {
    w: Option<u32>,
    h: Option<u32>,
    rgb: Option<String>,
    quality: Option<u8>,
    crop: Option<bool>,
    webp: Option<bool>,
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

    pub fn use_webp(&self) -> bool {
        self.webp.is_some_and(|v| v)
    }

    pub fn as_is(&self) -> bool {
        self.dimensions().is_none() && !self.use_webp()
    }

    pub fn unsupported_scale_size(&self) -> bool {
        let w = self.w.map_or(100, |v| v);
        let h = self.h.map_or(100, |v| v);
        !(20..=2000).contains(&w) || !(20..=1000).contains(&h)
    }
}
