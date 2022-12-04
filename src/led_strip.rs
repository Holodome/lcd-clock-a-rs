use crate::misc::{hsv2rgb_u8, ColorRGB8, Sin};

const LED_COUNT: usize = 6;
const DEFAULT_BRIGHTNESS: u8 = 0x40;

#[derive(Clone, Copy, Debug, Default)]
enum LedMode {
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

    pub fn left(&mut self) {
        self.mode = self.mode.left();
        self.transition = true;
    }

    pub fn right(&mut self) {
        self.mode = self.mode.right();
        self.transition = true;
    }

    pub fn update(&mut self) -> &[ColorRGB8; LED_COUNT] {
        if self.transition {
            self.transition = false;
            match self.mode {
                LedMode::Sin => {
                    self.colors = [Default::default(); LED_COUNT];
                    self.t = 0.0;
                }
                LedMode::Off => self.colors = [Default::default(); LED_COUNT],
                LedMode::Red => self.colors = [ColorRGB8::red(); LED_COUNT],
                LedMode::Green => self.colors = [ColorRGB8::green(); LED_COUNT],
                LedMode::Blue => self.colors = [ColorRGB8::blue(); LED_COUNT],
                LedMode::Cyan => self.colors = [ColorRGB8::cyan(); LED_COUNT],
                LedMode::Yellow => self.colors = [ColorRGB8::yellow(); LED_COUNT],
                LedMode::Pink => self.colors = [ColorRGB8::pink(); LED_COUNT],
            }
        }

        if let LedMode::Sin = self.mode {
            for (i, led) in self.colors.iter_mut().enumerate() {
                // An offset to give 3 consecutive LEDs a different color:
                let hue_offs = match i % 3 {
                    1 => 0.25,
                    2 => 0.5,
                    _ => 0.0,
                };

                let sin_11 = (self.sin)((self.t + hue_offs) * 2.0 * core::f32::consts::PI);
                let sin_01 = (sin_11 + 1.0) * 0.5;

                let hue = 360.0 * sin_01;
                let sat = 1.0;
                let val = 1.0;

                let rgb = hsv2rgb_u8(hue, sat, val);
                let rgb = (
                    ((rgb.0 as u16 * self.brightness as u16) / 0xff) as u8,
                    ((rgb.1 as u16 * self.brightness as u16) / 0xff) as u8,
                    ((rgb.2 as u16 * self.brightness as u16) / 0xff) as u8,
                );
                *led = rgb.into();
            }

            self.t += (16.0 / 1000.0) * self.animation_speed;
            while self.t > 1.0 {
                self.t -= 1.0;
            }
        }
        &self.colors
    }
}
