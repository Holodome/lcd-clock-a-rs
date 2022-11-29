//! DS3231 RTC
//! DS3231 Uses SMBUS which is a modified version of I2C. The only difference,
//! however, is only payload-connected: each transmission has to include the
//! command number. Otherwise it uses plain I2C.

use embedded_hal::blocking::i2c::{Write, WriteRead};

/// Temperature as acquired from rtc. It consists of 2 parts - 8 bits of degree
/// celcius and 2 bits of quartes of a degree. Because we have no FPU delay the
/// construction of float until usage/presentation of temperature and store it
/// as integer.
pub struct Temperature(u16);

impl Temperature {
    pub fn as_celcius(self) -> f32 {
        (self.0 >> 2) as f32 + (self.0 & 0x3) as f32 * 0.25
    }
}

/// Day of week
#[derive(Debug, Clone, Copy)]
pub enum Day {
    Sunday = 1,
    Monday = 2,
    Tuesday = 3,
    Wednesday = 4,
    Thursday = 5,
    Friday = 6,
    Saturday = 7,
}

impl TryFrom<u8> for Day {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use Day::*;
        match value {
            1 => Ok(Sunday),
            2 => Ok(Monday),
            3 => Ok(Tuesday),
            4 => Ok(Wednesday),
            5 => Ok(Thursday),
            6 => Ok(Friday),
            7 => Ok(Saturday),
            _ => Err(Error::DaysRange),
        }
    }
}

impl From<Day> for u8 {
    fn from(value: Day) -> Self {
        value as u8
    }
}

#[derive(Debug)]
pub struct Calendar {
    pub year: u16,
    pub month: u8,
    pub date: u8,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct Time {
    pub hours: u8,
    pub mins: u8,
    pub secs: u8,
}

pub struct DS3231State {
    addr: u8,
}

impl DS3231State {
    pub fn new(addr: u8) -> Self {
        Self { addr }
    }
}

/// DS3231 Driver
pub struct DS3231<I2C> {
    i2c: I2C,
    state: DS3231State,
}

impl<I2C> DS3231<I2C> {
    pub fn new(i2c: I2C, state: DS3231State) -> Self {
        Self { i2c, state }
    }

    pub fn release(self) -> (I2C, DS3231State) {
        (self.i2c, self.state)
    }
}

impl<I2C> DS3231<I2C>
where
    I2C: Write + WriteRead,
{
    pub fn init(&mut self) -> Result<(), Error> {
        // Enable tracking of temperature
        let status = self.read_reg(Register::Control)? | TEMP_BIT;
        self.write_reg(Register::Control, status)
    }

    fn read_reg(&mut self, reg: Register) -> Result<u8, Error> {
        let src = [reg as u8];
        let mut dst = [0u8];
        self.i2c
            .write_read(self.state.addr, &src, &mut dst)
            .map_err(|_| Error::BusRead)?;

        Ok(dst[0])
    }

    fn write_reg(&mut self, reg: Register, value: u8) -> Result<(), Error> {
        let buf = [reg as u8, value];
        self.i2c
            .write(self.state.addr, &buf)
            .map_err(|_| Error::BusWrite)
    }

    pub fn get_secs(&mut self) -> Result<u8, Error> {
        let secs = self.read_reg(Register::Seconds)?;
        Ok(secs.bcd_to_dec())
    }

    pub fn set_secs(&mut self, secs: u8) -> Result<(), Error> {
        if (0..=59).contains(&secs) {
            self.write_reg(Register::Seconds, secs.dec_to_bsd())
        } else {
            Err(Error::SecondsRange)
        }
    }

    pub fn get_mins(&mut self) -> Result<u8, Error> {
        let mins = self.read_reg(Register::Minutes)?;
        Ok(mins.bcd_to_dec())
    }

    pub fn set_mins(&mut self, mins: u8) -> Result<(), Error> {
        if (0..=59).contains(&mins) {
            self.write_reg(Register::Minutes, mins.dec_to_bsd())
        } else {
            Err(Error::MinutesRange)
        }
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
        if !(0..=23).contains(&hours) {
            return Err(Error::HoursRange);
        }

        let mode = extract_hour_info(self.read_reg(Register::Hours)?);
        let hours = match mode {
            HourInfo::H12PM | HourInfo::H12AM => {
                H12_BIT | if hours >= 12 { PM_BIT } else { 0 } | hours % 12
            }
            HourInfo::H24 => hours.dec_to_bsd(),
        };

        self.write_reg(Register::Hours, hours)
    }

    pub fn get_days(&mut self) -> Result<Day, Error> {
        let days = self.read_reg(Register::Days)?;
        days.try_into()
    }

    pub fn set_days(&mut self, days: Day) -> Result<(), Error> {
        self.write_reg(Register::Days, days.into())
    }

    pub fn get_date(&mut self) -> Result<u8, Error> {
        self.read_reg(Register::Days).map(|d| d.bcd_to_dec())
    }

    pub fn set_date(&mut self, date: u8) -> Result<(), Error> {
        if (0..31).contains(&date) {
            self.write_reg(Register::Date, date.dec_to_bsd())
        } else {
            Err(Error::DateRange)
        }
    }

    pub fn get_month(&mut self) -> Result<u8, Error> {
        self.read_reg(Register::Month).map(|m| m & MONTH_MASK)
    }

    pub fn set_month(&mut self, month: u8) -> Result<(), Error> {
        let century_bit = self.read_reg(Register::Month)? & CENTURY_BIT;
        if (1..12).contains(&month) {
            self.write_reg(Register::Month, month | century_bit)
        } else {
            Err(Error::MonthRange)
        }
    }

    pub fn get_year(&mut self) -> Result<u16, Error> {
        let century_bit = self.read_reg(Register::Month)? & CENTURY_BIT;
        self.read_reg(Register::Year)
            .map(|y| y.bcd_to_dec() as u16 + if century_bit != 0 { 100 } else { 0 } + YEAR_OFFSET)
    }

    pub fn set_year(&mut self, year: u16) -> Result<(), Error> {
        if (1900..=2099).contains(&year) {
            let year = (year - YEAR_OFFSET) as u8;
            let month_reg = self.read_reg(Register::Month)? & !CENTURY_BIT
                | if year >= 100 { CENTURY_BIT } else { 0 };
            self.write_reg(Register::Month, month_reg)?;
            let year = year % 100;

            self.write_reg(Register::Year, year.dec_to_bsd())
        } else {
            Err(Error::YearRange)
        }
    }

    pub fn get_temperature(&mut self) -> Result<Temperature, Error> {
        let high = self.read_reg(Register::TemperatureMSB)? as u16;
        let low = self.read_reg(Register::TemperatureLSB)? as u16;
        Ok(Temperature(high << 2 | (low >> 6)))
    }

    pub fn get_calendar(&mut self) -> Result<Calendar, Error> {
        let year = self.get_year()?;
        let month = self.get_month()?;
        let date = self.get_date()?;
        Ok(Calendar { year, month, date })
    }

    pub fn get_time(&mut self) -> Result<Time, Error> {
        let hours = self.get_hours()?;
        let mins = self.get_mins()?;
        let secs = self.get_secs()?;
        Ok(Time { hours, mins, secs })
    }
}

trait Bcd2Dec<T> {
    fn bcd_to_dec(self) -> T;
    fn dec_to_bsd(self) -> T;
}

impl Bcd2Dec<u8> for u8 {
    fn bcd_to_dec(self) -> u8 {
        (((self & 0xF0) >> 4) * 10) + (self & 0x0F)
    }

    fn dec_to_bsd(self) -> u8 {
        ((self / 10) << 4) | (self % 10)
    }
}

const H12_BIT: u8 = 0x40; // bit 6
const PM_BIT: u8 = 0x20; // bit 5
const H12_MASK: u8 = 0x0F; // bits 3-0 in 12 hours mode
const H24_MASK: u8 = 0x3F; // bits 5-0 in 24 hours mode is BCD
const CENTURY_BIT: u8 = 0x80; // bit 7
const MONTH_MASK: u8 = 0x0F;
const YEAR_OFFSET: u16 = 1900;
const TEMP_BIT: u8 = 0x20;

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

    SecondsRange,
    MinutesRange,
    HoursRange,
    DaysRange,
    DateRange,
    MonthRange,
    YearRange,
}

enum Register {
    Seconds = 0x00,
    Minutes = 0x01,
    Hours = 0x02,
    Days = 0x03,
    Date = 0x04,
    Month = 0x05,
    Year = 0x06,

    Control = 0x0E,

    TemperatureMSB = 0x11,
    TemperatureLSB = 0x12,
}
