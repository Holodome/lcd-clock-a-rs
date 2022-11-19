//! DS3231 RTC
//! DS3231 Uses SMBUS which is a modified version of I2C. The only difference,
//! however, is only payload-connected: each transmission has to include the
//! command number. Otherwise it uses plain I2C.

use embedded_hal::blocking::i2c::{Write, WriteRead};

/// This address is specified in schematic for product.
pub const ADDRESS: u16 = 0x68;

#[derive(Debug)]
pub struct Calendar {
    year: u16,
    month: u8,
    day: u8,
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

    pub fn get_seconds(&mut self) -> Result<u8, Error> {
        let secs = self.read_reg(Register::Seconds)?;
        Ok(secs.bcd_to_dec())
    }

    pub fn set_seconds(&mut self, secs: u8) -> Result<(), Error> {
        self.write_reg(Register::Seconds, secs.dec_to_bsd())
    }

    pub fn get_minutes(&mut self) -> Result<u8, Error> {
        let mins = self.read_reg(Register::Minutes)?;
        Ok(mins.bcd_to_dec())
    }

    pub fn set_minutes(&mut self, mins: u8) -> Result<(), Error> {
        self.write_reg(Register::Minutes, mins.dec_to_bsd())
    }

    fn set_hour_info(&mut self, info: HourInfo) -> Result<(), Error> {
        let hour = self.read_reg(Register::Hours)? & !H12_BIT & !PM_BIT;

        let hour = match info {
            HourInfo::H12PM => hour | H12_BIT | PM_BIT,
            HourInfo::H12AM => hour | H12_BIT,
            HourInfo::H24 => hour,
        };

        self.write_reg(Register::Hours, hour)
    }

    pub fn get_hours(&mut self) -> Result<u8, Error> {
        let hours = self.read_reg(Register::Hours)?;
        let mode = extract_hour_info(hours);
        let hours = match mode {
            HourInfo::H12PM => 12 + (hours & H12_MASK),
            HourInfo::H12AM => hours & H12_MASK,
            HourInfo::H24 => (hours & H24_MASK).bcd_to_dec(),
        };

        Ok(hours)
    }

    pub fn set_hours(&mut self, hours: u8) -> Result<(), Error> {
        let mode = extract_hour_info(self.read_reg(Register::Hours)?);
        let hours = match mode {
            HourInfo::H12PM | HourInfo::H12AM => {
                H12_BIT | if hours > 12 { PM_BIT } else { 0 } | hours % 12
            }
            HourInfo::H24 => hours.dec_to_bsd(),
        };

        self.write_reg(Register::Hours, hours)
    }
}

trait Bcd2Dec<T> {
    fn bcd_to_dec(self) -> T;
    fn dec_to_bsd(self) -> T;
}

impl Bcd2Dec<u8> for u8 {
    fn bcd_to_dec(self) -> u8 {
        (((self & 0xF0) >> 4) * 10) | (self & 0xF)
    }

    fn dec_to_bsd(self) -> u8 {
        ((self / 10) << 4) | (self % 10)
    }
}

const H12_BIT: u8 = 0x40; // bit 6
const PM_BIT: u8 = 0x20; // bit 5
const H12_MASK: u8 = 0x0F; // bits 3-0 in 12 hours mode
const H24_MASK: u8 = 0x3F; // bits 5-0 in 24 hours mode is BCD

fn extract_hour_info(hours: u8) -> HourInfo {
    if hours & H12_BIT != 0 {
        if hours & PM_BIT != 0 {
            HourInfo::H12PM
        } else {
            HourInfo::H12AM
        }
    } else {
        HourInfo::H24
    }
}

enum HourInfo {
    H24,
    H12AM,
    H12PM,
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
