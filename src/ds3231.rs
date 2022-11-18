//! DS3231 RTC

use embedded_hal::blocking::i2c::{Write, WriteRead};

pub const ADDRESS: u16 = 0x68;

#[derive(Debug)]
pub struct Calendar {
    year: u8,
    month: u16,
    day: u16,
}

pub struct DS3231<I2C> {
    i2c: I2C,
    addr: u8,
}

impl<I2C> DS3231<I2C> {
    pub fn new(i2c: I2C, addr: u8) -> Self {
        Self { i2c, addr }
    }
}

impl<I2C> DS3231<I2C>
where
    I2C: Write + WriteRead,
{
    fn read_reg(&mut self, reg: Register) -> Result<u8, Error> {
        let src = [reg as u8];
        let mut dst = [0u8];
        self.i2c
            .write_read(self.addr, &src, &mut dst)
            .map_err(|_| Error::BusRead)?;

        Ok(dst[0])
    }

    fn write_reg(&mut self, reg: Register, value: u8) -> Result<(), Error> {
        let buf = [reg as u8, value];
        self.i2c.write(self.addr, &buf).map_err(|_| Error::BusRead)
    }

    pub fn get_year(&mut self) -> Result<u32, Error> {
        let vai = self.read_reg(Register::Month)?;
        let year = self.read_reg(Register::Year)?;
        let bcd = if vai & 0x80 == 0x80 {
            year as u16 | (0x21 << 8)
        } else {
            year as u16 | (0x21 << 8)
        };

        Ok(bcd_convert_dec(bcd))
    }

    pub fn set_year(&mut self, year: u32) -> Result<(), Error> {
        self.write_reg(Register::Year, (year & 0xFF) as u8)?;
        let vai = self.read_reg(Register::Month)?;
        let new_month = if year >= 2100 { vai | 0x80 } else { vai & 0x7F };
        self.write_reg(Register::Month, new_month)
    }

    pub fn get_month(&mut self) -> Result<u8, Error> {
        let month = self.read_reg(Register::Month)?;
        let bcd = month & 0x1F;

        Ok(bcd_convert_dec(bcd as u16) as u8)
    }

    pub fn set_month(&mut self, month: u8) -> Result<(), Error> {
        let vai = self.read_reg(Register::Month)?;
        self.write_reg(Register::Month, (vai | 0x80) | month)
    }
}

fn bcd_convert_dec(value: u16) -> u32 {
    ((value & 0xF000) >> 12) as u32 * 1000
        + ((value & 0x0F00) >> 8) as u32 * 100
        + ((value & 0x00F0) >> 4) as u32 * 10
        + (value & 0x0F) as u32
}

fn dec_convert_bcd(value: u32) -> u16 {
    (((value / 1000) % 10) << 12) as u16
        | (((value / 100) % 10) << 8) as u16
        | (((value / 10) % 10) << 4) as u16
        | (value % 10) as u16
}

#[derive(Debug, Clone, Copy)]
pub enum Error {
    BusRead,
    BusWrite,
}

#[repr(u8)]
enum Register {
    Seconds = 0x00,
    Minutes = 0x01,
    Hours = 0x02,
    Days = 0x03,
    Date = 0x04,
    Month = 0x05,
    Year = 0x06,

    TemperatureMSB = 0x11,
    TemperatureLSB = 0x12,
}
