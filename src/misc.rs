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

pub struct ColorRGB565(pub u16);

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
