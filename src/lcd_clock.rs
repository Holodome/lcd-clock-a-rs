//! General project-wide functionality

use crate::{
    drivers::{
        bme280::{self, BME280State, BME280},
        buttons::{Button, ButtonEvent},
        ds3231::{self, DS3231State, Time, DS3231},
        st7789vwx6::{self, Display, ST7789VWx6},
        ws2812::WS2812,
    },
    images::{MENUPIC_A, NUMPIC_A},
    led_strip::{LedMode, LedStripState, LED_COUNT},
    misc::{ColorRGB565, ColorRGB8, Sin},
};

use crate::hal::{
    gpio::{
        bank0::{Gpio12, Gpio15, Gpio16, Gpio17, Gpio2, Gpio22, Gpio3, Gpio4, Gpio6, Gpio7, Gpio8},
        FunctionI2C, Pin, PullDownInput, PushPullOutput,
    },
    i2c::I2C,
    pac::{I2C1, PIO0, SPI1},
    pio::SM0,
    pwm::{self, Pwm6},
    spi::{self, Spi},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AppMode {
    #[default]
    Time,
    Alarm,
    Rgb,
    Brightness,
    TempHumidity,
    Return,
}

impl AppMode {
    pub fn left(self) -> Self {
        match self {
            Self::Time => Self::Return,
            Self::Alarm => Self::Time,
            Self::Rgb => Self::Alarm,
            Self::Brightness => Self::Rgb,
            Self::TempHumidity => Self::Brightness,
            Self::Return => Self::TempHumidity,
        }
    }

    pub fn right(self) -> Self {
        match self {
            Self::Time => Self::Alarm,
            Self::Alarm => Self::Rgb,
            Self::Rgb => Self::Brightness,
            Self::Brightness => Self::TempHumidity,
            Self::TempHumidity => Self::Return,
            Self::Return => Self::Time,
        }
    }

    pub fn all() -> impl Iterator<Item = Self> {
        [
            Self::Time,
            Self::Alarm,
            Self::Rgb,
            Self::Brightness,
            Self::TempHumidity,
            Self::Return,
        ]
        .iter()
        .copied()
    }
}

type I2CBusTy = I2C<I2C1, (Pin<Gpio6, FunctionI2C>, Pin<Gpio7, FunctionI2C>)>;
type ST7789VWx6Ty = ST7789VWx6<
    (
        Pin<Gpio2, PushPullOutput>,
        Pin<Gpio3, PushPullOutput>,
        Pin<Gpio4, PushPullOutput>,
        Pin<Gpio8, PushPullOutput>,
        Pin<Gpio12, PushPullOutput>,
    ),
    Spi<spi::Enabled, SPI1, 8>,
    pwm::Channel<Pwm6, pwm::FreeRunning, pwm::B>,
>;
type WS2812Ty = WS2812<PIO0, SM0, Gpio22>;
type DS3231Ty = DS3231<I2CBusTy>;
type BME280Ty = BME280<I2CBusTy>;

type LeftBtnTy = Button<Pin<Gpio15, PullDownInput>>;
type RightBtnTy = Button<Pin<Gpio16, PullDownInput>>;
type ModeBtnTy = Button<Pin<Gpio17, PullDownInput>>;
type BuzzerTy = ();

pub struct LcdClockHardware {
    i2c_bus: Option<I2CBusTy>,
    rtc: Option<DS3231State>,
    humidity_sensor: Option<BME280State>,
    displays: ST7789VWx6Ty,
    led_strip: WS2812Ty,
    buzzer: BuzzerTy,
    left: LeftBtnTy,
    right: RightBtnTy,
    mode: ModeBtnTy,
}

impl LcdClockHardware {
    pub fn new(
        i2c_bus: I2CBusTy,
        displays: ST7789VWx6Ty,
        led_strip: WS2812Ty,
        left: LeftBtnTy,
        right: RightBtnTy,
        mode: ModeBtnTy,
        buzzer: BuzzerTy,
    ) -> Self {
        Self {
            i2c_bus: Some(i2c_bus),
            rtc: None,
            humidity_sensor: None,
            displays,
            led_strip,
            left,
            right,
            mode,
            buzzer,
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.rtc.replace(DS3231State::new(DS3231_I2C_ADDR));
        self.humidity_sensor
            .replace(BME280State::new(BME280_I2C_ADDR));
        self.with_rtc(DS3231Ty::init)?.map_err(Error::Rtc)?;
        self.with_humidity_sensor(BME280Ty::init)?
            .map_err(Error::HumiditySensor)?;
        self.displays.init().map_err(Error::Display)?;
        self.display_clear_all(ColorRGB565::from(ColorRGB8::black()))?;

        Ok(())
    }

    /// Calls f on instance of ds3231. I2C bus is shared between ds3231 and
    /// bme280 drivers and rust type system forbids us from using two
    /// drivers simultaneosly. Thus i2c_bus field acts like a mutex.
    fn with_rtc<R>(&mut self, f: impl FnOnce(&mut DS3231Ty) -> R) -> Result<R, Error> {
        if self.i2c_bus.is_none() || self.rtc.is_none() {
            return Err(Error::I2CClaim);
        }

        let (Some(i2c_bus), Some(ds3231_state)) = (self.i2c_bus.take(), self.rtc.take()) else {
            return Err(Error::I2CClaim);
        };

        let mut ds3231 = DS3231Ty::new(i2c_bus, ds3231_state);
        let result = f(&mut ds3231);
        let (i2c_bus, ds3231_state) = ds3231.release();
        self.i2c_bus.replace(i2c_bus);
        self.rtc.replace(ds3231_state);
        Ok(result)
    }

    /// Calls f on instance of bme280. For details see with_ds3231.
    fn with_humidity_sensor<R>(&mut self, f: impl FnOnce(&mut BME280Ty) -> R) -> Result<R, Error> {
        if self.i2c_bus.is_none() || self.humidity_sensor.is_none() {
            return Err(Error::I2CClaim);
        }

        let (Some(i2c_bus), Some(bme280_state)) = (self.i2c_bus.take(), self.humidity_sensor.take()) else {
            return Err(Error::I2CClaim);
        };

        let mut bme280 = BME280Ty::new(i2c_bus, bme280_state);
        let result = f(&mut bme280);
        let (i2c_bus, bme280_state) = bme280.release();
        self.i2c_bus.replace(i2c_bus);
        self.humidity_sensor.replace(bme280_state);
        Ok(result)
    }

    fn display_fill(&mut self, display: Display, color: ColorRGB565) -> Result<(), Error> {
        let w = self.displays.width();
        let h = self.displays.height();
        self.displays
            .set_pixels_iter(
                display,
                0,
                0,
                w,
                h,
                (0..(w * h)).flat_map(|_| color.to_be()),
            )
            .map_err(Error::Display)
    }

    fn display_clear_all(&mut self, color: ColorRGB565) -> Result<(), Error> {
        for display in Display::all() {
            self.display_fill(display, color)?;
        }

        Ok(())
    }

    fn display_draw_rect(
        &mut self,
        display: Display,
        x_min: u16,
        y_min: u16,
        x_max: u16,
        y_max: u16,
        color: ColorRGB565,
    ) -> Result<(), Error> {
        self.displays
            .set_pixels_iter(
                display,
                x_min,
                y_min,
                x_max,
                y_max,
                (0..((x_max - x_min) * (y_max - y_min))).flat_map(|_| color.to_be()),
            )
            .map_err(Error::Display)
    }
}

struct State {
    mode: AppMode,
    // if we are in menu, this is 'Some'
    menu: Option<AppMode>,
    led_strip: LedStripState,

    transition: bool,
}

impl State {
    pub fn new(sin: Sin) -> Self {
        Self {
            mode: AppMode::Time,
            led_strip: LedStripState::new(sin),
            menu: None,
            transition: true,
        }
    }

    fn eat_transition(&mut self) -> bool {
        let result = self.transition;
        self.transition = false;
        result
    }

    fn handle_buttons(
        &mut self,
        mode: Option<ButtonEvent>,
        left: Option<ButtonEvent>,
        right: Option<ButtonEvent>,
    ) {
        let mode = matches!(mode, Some(ButtonEvent::Release));
        let left = matches!(left, Some(ButtonEvent::Release));
        let right = matches!(right, Some(ButtonEvent::Release));
        if let Some(menu) = self.menu.take() {
            if mode {
                if let AppMode::Return = menu {
                } else {
                    self.mode = menu;
                }
                self.transition = true;
            } else if left {
                self.menu = Some(menu.left());
                self.transition = true;
            } else if right {
                self.menu = Some(menu.right());
                self.transition = true;
            } else {
                self.menu = Some(menu);
            }
        } else if mode {
            self.menu = Some(AppMode::Return);
            self.transition = true;
        } else {
            match self.mode {
                AppMode::Rgb => {
                    if left {
                        self.led_strip.left();
                        self.transition = true;
                    }
                    if right {
                        self.led_strip.right();
                        self.transition = true;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn update(&mut self) {
        self.led_strip.update();
    }
}

pub struct LcdClock {
    hardware: LcdClockHardware,
    state: State,

    /// Used as comparator value needed to decide which displays we want to
    /// update
    last_time: Time,
}

impl LcdClock {
    pub fn new(hardware: LcdClockHardware, sin: Sin) -> Self {
        Self {
            hardware,
            state: State::new(sin),
            last_time: Default::default(),
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.hardware.init()?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), Error> {
        self.update_buttons();

        let transition = self.state.eat_transition();
        if let Some(menu_mode) = self.state.menu {
            if transition {
                self.mode_menu(menu_mode)?;
            }
        } else {
            let mode = self.state.mode;
            match mode {
                AppMode::Time => self.mode_time(transition)?,
                AppMode::Rgb => self.mode_rgb(transition)?,
                _ => todo!(),
            }
        }

        // TODO: dynamic update time (using rtc or system timer)
        cortex_m::asm::delay(125 * 1000 * 16);
        self.state.update();
        self.hardware
            .led_strip
            .display(self.state.led_strip.colors());

        Ok(())
    }

    fn mode_menu(&mut self, selected_mode: AppMode) -> Result<(), Error> {
        for (mode, display) in AppMode::all().zip(Display::all()) {
            let pic = MENUPIC_A.get_pic(mode);
            let w = pic.width() as u16;
            let h = pic.height() as u16;
            let pix = pic.pixels();
            self.hardware
                .displays
                .set_pixels(display, 0, 0, w, h, pix)
                .map_err(Error::Display)?;

            if mode == selected_mode {
                let w = self.hardware.displays.width();
                let h = self.hardware.displays.height();
                let thickness = 8;
                let color = ColorRGB565::from(ColorRGB8::red());
                self.hardware
                    .display_draw_rect(display, 0, 0, w, thickness, color)?;
                self.hardware
                    .display_draw_rect(display, 0, thickness, thickness, h, color)?;
                self.hardware
                    .display_draw_rect(display, w - thickness, thickness, w, h, color)?;
                self.hardware.display_draw_rect(
                    display,
                    thickness,
                    h - thickness,
                    w - thickness,
                    h,
                    color,
                )?;
            }
        }

        Ok(())
    }

    fn update_buttons(&mut self) {
        let mode_button_transition = self.hardware.mode.update();
        let left_button_transition = self.hardware.left.update();
        let right_button_transition = self.hardware.right.update();
        self.state.handle_buttons(
            mode_button_transition,
            left_button_transition,
            right_button_transition,
        );
    }

    fn mode_time(&mut self, force_update: bool) -> Result<(), Error> {
        let time = self
            .hardware
            .with_rtc(|rtc| rtc.get_time())?
            .map_err(Error::Rtc)?;

        let time_displays = time_to_display_values(time);
        let prev_time_displays = time_to_display_values(self.last_time);

        for ((display, &time), &prev) in Display::all()
            .into_iter()
            .zip(time_displays.iter())
            .zip(prev_time_displays.iter())
        {
            if let Some(pic) = NUMPIC_A.get_digit(time) {
                let w = pic.width() as u16;
                let h = pic.height() as u16;
                let pix = pic.pixels();
                if time != prev || force_update {
                    self.hardware
                        .displays
                        .set_pixels(display, 0, 0, w, h, pix)
                        .map_err(Error::Display)?;
                }
            }
        }

        self.last_time = time;

        Ok(())
    }

    fn mode_rgb(&mut self, force_update: bool) -> Result<(), Error> {
        let colors = match self.state.led_strip.mode() {
            LedMode::Sin => [
                ColorRGB8::red(),
                ColorRGB8::green(),
                ColorRGB8::blue(),
                ColorRGB8::cyan(),
                ColorRGB8::yellow(),
                ColorRGB8::pink(),
            ],
            LedMode::Off => [ColorRGB8::black(); LED_COUNT],
            LedMode::Red => [ColorRGB8::red(); LED_COUNT],
            LedMode::Green => [ColorRGB8::green(); LED_COUNT],
            LedMode::Blue => [ColorRGB8::blue(); LED_COUNT],
            LedMode::Cyan => [ColorRGB8::cyan(); LED_COUNT],
            LedMode::Yellow => [ColorRGB8::yellow(); LED_COUNT],
            LedMode::Pink => [ColorRGB8::pink(); LED_COUNT],
        };

        if force_update {
            for (display, color) in Display::all().zip(colors) {
                self.hardware.display_fill(display, color.into())?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {
    Display(st7789vwx6::Error),
    HumiditySensor(bme280::Error),
    Rtc(ds3231::Error),

    I2CClaim,
}

/// This addresses are specified in schematic for product.
pub const BME280_I2C_ADDR: u8 = 0x76;
pub const DS3231_I2C_ADDR: u8 = 0x68;

fn time_to_display_values(time: Time) -> [u8; 6] {
    let houra = time.hours / 10;
    let hourb = time.hours % 10;
    let mina = time.mins / 10;
    let minb = time.mins % 10;
    let seca = time.secs / 10;
    let secb = time.secs % 10;

    [houra, hourb, mina, minb, seca, secb]
}
