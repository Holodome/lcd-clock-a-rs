use crate::{
    drivers::{
        bme280::{BME280State, BME280},
        buttons::{Button, ButtonEvent},
        ds3231::{DS3231State, DS3231},
        st7789vwx6::ST7789VWx6,
        ws2812::WS2812,
    },
    gl::Gl,
    lcd_clock::Error,
    misc::{ColorRGB565, ColorRGB8},
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

pub type I2CBusTy = I2C<I2C1, (Pin<Gpio6, FunctionI2C>, Pin<Gpio7, FunctionI2C>)>;
pub type ST7789VWx6Ty = ST7789VWx6<
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
pub type WS2812Ty = WS2812<PIO0, SM0, Gpio22>;
pub type DS3231Ty = DS3231<I2CBusTy>;
pub type BME280Ty = BME280<I2CBusTy>;

pub type LeftBtnTy = Button<Pin<Gpio15, PullDownInput>>;
pub type RightBtnTy = Button<Pin<Gpio16, PullDownInput>>;
pub type ModeBtnTy = Button<Pin<Gpio17, PullDownInput>>;
pub type BuzzerTy = ();

pub struct LcdClockHardware {
    i2c_bus: Option<I2CBusTy>,
    rtc: Option<DS3231State>,
    humidity_sensor: Option<BME280State>,
    pub displays: ST7789VWx6Ty,
    pub led_strip: WS2812Ty,
    pub buzzer: BuzzerTy,
    pub left: LeftBtnTy,
    pub right: RightBtnTy,
    pub mode: ModeBtnTy,
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
        self.with_gl(|gl| gl.clear_all(ColorRGB565::from(ColorRGB8::black())))?;

        Ok(())
    }

    /// Calls f on instance of ds3231. I2C bus is shared between ds3231 and
    /// bme280 drivers and rust type system forbids us from using two
    /// drivers simultaneosly. Thus i2c_bus field acts like a mutex.
    pub fn with_rtc<R>(&mut self, f: impl FnOnce(&mut DS3231Ty) -> R) -> Result<R, Error> {
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
    pub fn with_humidity_sensor<R>(
        &mut self,
        f: impl FnOnce(&mut BME280Ty) -> R,
    ) -> Result<R, Error> {
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

    pub fn with_gl<R>(&mut self, f: impl FnOnce(&mut Gl) -> R) -> R {
        let mut gl = Gl::new(&mut self.displays);
        f(&mut gl)
    }

    pub fn update_buttons(
        &mut self,
    ) -> (
        Option<ButtonEvent>,
        Option<ButtonEvent>,
        Option<ButtonEvent>,
    ) {
        (self.mode.update(), self.left.update(), self.right.update())
    }
}

/// This addresses are specified in schematic for product.
pub const BME280_I2C_ADDR: u8 = 0x76;
pub const DS3231_I2C_ADDR: u8 = 0x68;
