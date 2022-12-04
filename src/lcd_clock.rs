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
};
use rp_pico::hal::gpio::PushPullOutput;

use crate::hal::{
    gpio::{
        bank0::{Gpio12, Gpio15, Gpio16, Gpio17, Gpio2, Gpio22, Gpio3, Gpio4, Gpio6, Gpio7, Gpio8},
        FunctionI2C, Pin, PullDownInput,
    },
    i2c::I2C,
    pac::{I2C1, PIO0, SPI1},
    pio::SM0,
    pwm::{self, Pwm6},
    spi::{self, Spi},
};

#[derive(Clone, Copy, Debug, Default)]
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
        self.displays.clear_all(0).map_err(Error::Display)?;

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
}

struct State {
    mode: AppMode,
    // if we are in menu, this is 'Some'
    menu: Option<AppMode>,

    transition: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            mode: AppMode::Time,
            menu: None,
            transition: true,
        }
    }
}

impl State {
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
        if let Some(menu) = self.menu.take() {
            if let Some(ButtonEvent::Release) = mode {
                if let AppMode::Return = menu {
                } else {
                    self.mode = menu;
                }
                self.transition = true;
            } else if let Some(ButtonEvent::Release) = left {
                self.menu = Some(menu.left());
                self.transition = true;
            } else if let Some(ButtonEvent::Release) = right {
                self.menu = Some(menu.right());
                self.transition = true;
            } else {
                self.menu = Some(menu);
            }
        } else {
            if let Some(ButtonEvent::Release) = mode {
                self.menu = Some(self.mode);
            }
        }
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
    pub fn new(hardware: LcdClockHardware) -> Self {
        Self {
            hardware,
            state: Default::default(),

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
            self.mode_menu(menu_mode)?;
        } else {
            let mode = self.state.mode;
            match mode {
                AppMode::Time => self.mode_time(transition)?,
                _ => todo!(),
            }
        }

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
