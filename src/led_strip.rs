use crate::misc::{hsv2rgb_u8, ColorRGB8, Sin};

pub const LED_COUNT: usize = 6;
const DEFAULT_BRIGHTNESS: u8 = 0x40;

#[derive(Clone, Copy, Debug, Default)]
pub enum LedMode {
    Off,
    #[default]
    Sin,
    Red,
    Green,
    Blue,
    Yellow,
    Cyan,
    Pink,
}

impl LedMode {
    fn right(self) -> Self {
        match self {
            Self::Off => Self::Sin,
            Self::Sin => Self::Red,
            Self::Red => Self::Green,
            Self::Green => Self::Blue,
            Self::Blue => Self::Yellow,
            Self::Yellow => Self::Cyan,
            Self::Cyan => Self::Pink,
            Self::Pink => Self::Off,
        }
    }

    fn left(self) -> Self {
        match self {
            Self::Off => Self::Pink,
            Self::Sin => Self::Off,
            Self::Red => Self::Sin,
            Self::Green => Self::Red,
            Self::Blue => Self::Green,
            Self::Yellow => Self::Blue,
            Self::Cyan => Self::Yellow,
            Self::Pink => Self::Cyan,
        }
    }
}

pub struct LedStripState {
    colors: [ColorRGB8; LED_COUNT],
    mode: LedMode,
    transition: bool,

    sin: Sin,

    brightness: u8,
    t: f32,
    animation_speed: f32,
}

impl LedStripState {
    pub fn new(sin: Sin) -> Self {
        Self {
            colors: [Default::default(); LED_COUNT],
            mode: Default::default(),
            transition: false,
            sin,
            brightness: DEFAULT_BRIGHTNESS,
            t: 0.0,
            animation_speed: 0.1,
        }
    }

    pub fn mode(&self) -> LedMode {
        self.mode
    }

    pub fn left(&mut self) {
        self.mode = self.mode.left();
        self.transition = true;
    }

    pub fn right(&mut self) {
        self.mode = self.mode.right();
        self.transition = true;
    }

    pub fn colors(&self) -> &[ColorRGB8; LED_COUNT] {
        &self.colors
    }

    pub fn update(&mut self) {
        if self.transition {
            self.transition = false;
            let colors = match self.mode {
                LedMode::Sin => {
                    self.t = 0.0;
                    [Default::default(); LED_COUNT]
                }
                LedMode::Off => [Default::default(); LED_COUNT],
                LedMode::Red => [ColorRGB8::red(); LED_COUNT],
                LedMode::Green => [ColorRGB8::green(); LED_COUNT],
                LedMode::Blue => [ColorRGB8::blue(); LED_COUNT],
                LedMode::Cyan => [ColorRGB8::cyan(); LED_COUNT],
                LedMode::Yellow => [ColorRGB8::yellow(); LED_COUNT],
                LedMode::Pink => [ColorRGB8::pink(); LED_COUNT],
            };

            self.colors = colors.map(|color| adjust_brightness(color, self.brightness));
        }

        if let LedMode::Sin = self.mode {
            for (i, led) in self.colors.iter_mut().enumerate() {
                // An offset to give 6 consecutive LEDs a different color:
                let max_offs = 0.5;
                let modulo = i % LED_COUNT;
                let hue_offs = if modulo != 0 {
                    max_offs / modulo as f32
                } else {
                    0.0
                };

                let sin_11 = (self.sin)((self.t + hue_offs) * core::f32::consts::TAU);
                let sin_01 = (sin_11 + 1.0) * 0.5;

                let hue = 360.0 * sin_01;
                let sat = 1.0;
                let val = 1.0;

                let rgb = hsv2rgb_u8(hue, sat, val);
                *led = adjust_brightness(rgb.into(), self.brightness);
            }

            self.t += (16.0 / 1000.0) * self.animation_speed;
            while self.t > 1.0 {
                self.t -= 1.0;
            }
        }
    }
}

fn adjust_brightness(color: ColorRGB8, brightness: u8) -> ColorRGB8 {
    let rgb = (
        ((color.r as u16 * brightness as u16) / 0xff) as u8,
        ((color.g as u16 * brightness as u16) / 0xff) as u8,
        ((color.b as u16 * brightness as u16) / 0xff) as u8,
    );

    rgb.into()
}
