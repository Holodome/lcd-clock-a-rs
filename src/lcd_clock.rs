//! General project-wide functionality

use crate::drivers::{
    bme280::{self, BME280State, BME280},
    ds3231::{self, DS3231State, Time, DS3231},
    st7789vwx6::{self, Display, ST7789VWx6},
    ws2812::WS2812,
};
use rp_pico::hal::gpio::PushPullOutput;

use crate::hal::{
    gpio::{
        bank0::{Gpio12, Gpio2, Gpio22, Gpio3, Gpio4, Gpio6, Gpio7, Gpio8},
        FunctionI2C, Pin,
    },
    i2c::I2C,
    pac::{I2C1, PIO0, SPI1},
    pio::SM0,
    pwm::{self, Pwm6},
    spi::{self, Spi},
};

#[derive(Clone, Copy, Debug, Default)]
pub enum MenuMode {
    #[default]
    Time,
    Alarm,
    Rgb,
    Brightness,
    TempHumidity,
    Return,
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

type BuzzerTy = ();

pub struct LcdClockHardware {
    i2c_bus: Option<I2CBusTy>,
    ds3231: Option<DS3231State>,
    bme280: Option<BME280State>,
    st7789vwx6: ST7789VWx6Ty,
    ws2812: WS2812Ty,
    buzzer: BuzzerTy,
}

impl LcdClockHardware {
    pub fn new(
        i2c_bus: I2CBusTy,
        st7789vwx6: ST7789VWx6Ty,
        ws2812: WS2812Ty,
        buzzer: BuzzerTy,
    ) -> Self {
        Self {
            i2c_bus: Some(i2c_bus),
            ds3231: None,
            bme280: None,
            st7789vwx6,
            ws2812,
            buzzer,
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.ds3231.replace(DS3231State::new(DS3231_I2C_ADDR));
        self.bme280.replace(BME280State::new(BME280_I2C_ADDR));
        self.with_ds3231(DS3231Ty::init)?.map_err(Error::Rtc)?;
        self.with_bme280(BME280Ty::init)?
            .map_err(Error::HumiditySensor)?;
        self.st7789vwx6.init().map_err(Error::Display)?;
        self.st7789vwx6.clear_all(0).map_err(Error::Display)?;

        Ok(())
    }

    /// Calls f on instance of ds3231. I2C bus is shared between ds3231 and
    /// bme280 drivers and rust type system forbids us from using two
    /// drivers simultaneosly. Thus i2c_bus field acts like a mutex.
    fn with_ds3231<R>(&mut self, f: impl FnOnce(&mut DS3231Ty) -> R) -> Result<R, Error> {
        if self.i2c_bus.is_none() || self.ds3231.is_none() {
            return Err(Error::I2CClaim);
        }

        let (Some(i2c_bus), Some(ds3231_state)) = (self.i2c_bus.take(), self.ds3231.take()) else {
            return Err(Error::I2CClaim);
        };

        let mut ds3231 = DS3231Ty::new(i2c_bus, ds3231_state);
        let result = f(&mut ds3231);
        let (i2c_bus, ds3231_state) = ds3231.release();
        self.i2c_bus.replace(i2c_bus);
        self.ds3231.replace(ds3231_state);
        Ok(result)
    }

    /// Calls f on instance of bme280. For details see with_ds3231.
    fn with_bme280<R>(&mut self, f: impl FnOnce(&mut BME280Ty) -> R) -> Result<R, Error> {
        let (Some(i2c_bus), Some(bme280_state)) = (self.i2c_bus.take(), self.bme280.take()) else {
            return Err(Error::I2CClaim);
        };

        let mut bme280 = BME280Ty::new(i2c_bus, bme280_state);
        let result = f(&mut bme280);
        let (i2c_bus, bme280_state) = bme280.release();
        self.i2c_bus.replace(i2c_bus);
        self.bme280.replace(bme280_state);
        Ok(result)
    }
}

pub struct LcdClock {
    hardware: LcdClockHardware,
    menu_mode: MenuMode,
}

impl LcdClock {
    pub fn new(hardware: LcdClockHardware) -> Self {
        Self {
            hardware,
            menu_mode: Default::default(),
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.hardware.init()?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), Error> {
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
