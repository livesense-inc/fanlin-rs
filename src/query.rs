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
        match self.rgb.as_ref() {
            Some(text) => {
                let rgb: Vec<u8> = text
                    .split(',')
                    .map(|e| match e.parse::<u8>() {
                        Ok(c) => c,
                        Err(_) => 32,
                    })
                    .collect();
                if rgb.len() != 3usize {
                    return (32, 32, 32);
                }
                (rgb[0], rgb[1], rgb[2])
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
