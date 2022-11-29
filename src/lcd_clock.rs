//! General project-wide functionality

use rp_pico::hal::{gpio::PushPullOutput, pwm::ChannelId, spi::SpiDevice};

use crate::{
    bme280::{BME280State, BME280},
    ds3231::{DS3231State, Time, DS3231},
    st7789vwx6::{Display, ST7789VWx6},
};

use crate::hal::{
    gpio::{
        bank0::{Gpio12, Gpio2, Gpio3, Gpio4, Gpio6, Gpio7, Gpio8},
        FunctionI2C, FunctionSpi, Pin,
    },
    i2c::I2C,
    pac::{I2C1, SPI1},
    pwm::{self, Pwm6},
    spi::{self, Spi},
};

pub enum MenuOpt {
    Time,
    Alarm,
    RGB,
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

pub struct LcdClock {
    i2c_bus: Option<I2CBusTy>,
    ds3231: DS3231State,
    bme280: BME280State,
    st7789vwx6: ST7789VWx6Ty,
}

impl LcdClock {
    pub fn new(
        i2c_bus: I2CBusTy,
        ds3231: DS3231State,
        bme280: BME280State,
        st7789vwx6: ST7789VWx6Ty,
    ) -> Self {
        Self {
            i2c_bus: Some(i2c_bus),
            ds3231,
            bme280,
            st7789vwx6,
        }
    }
}

/// This addresses are specified in schematic for product.
pub const BME280_I2C_ADDR: u8 = 0x76;
pub const DS3231_I2C_ADDR: u8 = 0x68;
