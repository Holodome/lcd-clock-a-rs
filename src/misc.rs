pub type Sin = extern "C" fn(f32) -> f32;

#[derive(Clone, Copy, Default)]
pub struct ColorRGB8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<(u8, u8, u8)> for ColorRGB8 {
    fn from(value: (u8, u8, u8)) -> Self {
        Self {
            r: value.0,
            g: value.1,
            b: value.2,
        }
    }
}

impl From<ColorRGB8> for (u8, u8, u8) {
    fn from(value: ColorRGB8) -> Self {
        (value.r, value.g, value.b)
    }
}

impl ColorRGB8 {
    pub fn black() -> Self {
        Self {
            r: 0x00,
            g: 0x00,
            b: 0x00,
        }
    }

    pub fn red() -> Self {
        Self {
            r: 0xff,
            g: 0x00,
            b: 0x00,
        }
    }

    pub fn green() -> Self {
        Self {
            r: 0x00,
            g: 0xff,
            b: 0x00,
        }
    }

    pub fn blue() -> Self {
        Self {
            r: 0x00,
            g: 0x00,
            b: 0xff,
        }
    }

    pub fn cyan() -> Self {
        Self {
            r: 0x00,
            g: 0xff,
            b: 0xff,
        }
    }

    pub fn yellow() -> Self {
        Self {
            r: 0xff,
            g: 0xff,
            b: 0x00,
        }
    }

    pub fn pink() -> Self {
        Self {
            r: 0xff,
            g: 0x00,
            b: 0xff,
        }
    }
}

/// Stores color in RGB565 format (big endian) so it is more suitable for using
/// in rendering on st7789 display (which uses be).
#[derive(Clone, Copy, Default)]
pub struct ColorRGB565(pub u16);

impl ColorRGB565 {
    pub fn to_be(self) -> [u8; 2] {
        self.0.to_be_bytes()
    }
}

impl From<u16> for ColorRGB565 {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<ColorRGB565> for u16 {
    fn from(value: ColorRGB565) -> Self {
        value.0
    }
}

impl From<ColorRGB8> for ColorRGB565 {
    fn from(value: ColorRGB8) -> Self {
        let r = (value.r >> 3) as u16;
        let g = (value.g >> 2) as u16;
        let b = (value.b >> 3) as u16;
        let rgb = (r << 11) | (g << 5) | b;
        Self(rgb)
    }
}

pub fn hsv2rgb(hue: f32, sat: f32, val: f32) -> (f32, f32, f32) {
    let c = val * sat;
    let v = (hue / 60.0) % 2.0 - 1.0;
    let v = if v < 0.0 { -v } else { v };
    let x = c * (1.0 - v);
    let m = val - c;
    let (r, g, b) = if hue < 60.0 {
        (c, x, 0.0)
    } else if hue < 120.0 {
        (x, c, 0.0)
    } else if hue < 180.0 {
        (0.0, c, x)
    } else if hue < 240.0 {
        (0.0, x, c)
    } else if hue < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (r + m, g + m, b + m)
}

pub fn hsv2rgb_u8(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let r = hsv2rgb(h, s, v);

    (
        (r.0 * 255.0) as u8,
        (r.1 * 255.0) as u8,
        (r.2 * 255.0) as u8,
    )
}
