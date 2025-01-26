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
        match &self.rgb {
            Some(rgb) => {
                let mut iter = rgb.split(',').map(|e| e.parse::<u8>().unwrap());
                (
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                    iter.next().unwrap(),
                )
            }
            None => (32, 32, 32),
        }
    }

    pub fn quality(&self) -> u8 {
        match self.quality {
            Some(v) => v,
            None => 85,
        }
    }

    pub fn cropping(&self) -> bool {
        match self.crop {
            Some(v) => v,
            None => false,
        }
    }

    pub fn use_webp(&self) -> bool {
        match self.webp {
            Some(v) => v,
            None => false,
        }
    }
}
