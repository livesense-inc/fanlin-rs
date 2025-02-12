//  0 0 0 0 0 0 0 0
//  | | | | | | | |
//  | | | | | | | +- webp
//  | | | | | | +--- avif
//  | | | | | +----- -
//  | | | | +------- -
//  | | | +--------- -
//  | | +----------- -
//  | +------------- -
//  +--------------- -

#[derive(Clone, Copy, Debug, Default)]
pub struct Format(u8);

const WEBP_FLAG: u8 = 1 << 0;
const AVIF_FLAG: u8 = 1 << 1;

impl Format {
    pub fn new() -> Self {
        Self(0u8)
    }

    pub fn accept_webp(&mut self) {
        self.accept(WEBP_FLAG);
    }

    pub fn webp_accepted(&self) -> bool {
        self.accepted(WEBP_FLAG)
    }

    pub fn accept_avif(&mut self) {
        self.accept(AVIF_FLAG);
    }

    pub fn avif_accepted(&self) -> bool {
        self.accepted(AVIF_FLAG)
    }

    #[inline]
    fn accept(&mut self, mask: u8) {
        self.0 |= mask
    }

    #[inline]
    fn accepted(&self, mask: u8) -> bool {
        (self.0 & mask) == mask
    }
}

#[test]
fn test_content_format_flag() {
    let mut format = Format::new();

    assert!(!format.webp_accepted());
    format.accept_webp();
    assert!(format.webp_accepted());

    assert!(!format.avif_accepted());
    format.accept_avif();
    assert!(format.avif_accepted());
}
