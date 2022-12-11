use core::borrow::BorrowMut;

use embedded_hal::blocking::i2c::{Write, WriteRead};

#[derive(Clone, Copy)]
pub struct Temperature(i32);

impl Temperature {
    pub fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    pub fn as_celcius(self) -> f32 {
        self.0 as f32 / 100.
    }
}

impl core::fmt::Debug for Temperature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Temperature")
            .field("celcius", &self.as_celcius())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct Pressure(u32);

impl Pressure {
    pub fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub fn as_pas(self) -> f32 {
        self.0 as f32 / 256.
    }
}

impl core::fmt::Debug for Pressure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Pressure")
            .field("pas", &self.as_pas())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct Humidity(u32);

impl Humidity {
    pub fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub fn as_percent(self) -> f32 {
        self.0 as f32 / 1024.
    }
}

impl core::fmt::Debug for Humidity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Humidity")
            .field("percents", &self.as_percent())
            .finish()
    }
}

pub struct BME280State {
    addr: u8,
    compensator: Option<ADCCompensator>,
}

impl BME280State {
    pub fn new(addr: u8) -> Self {
        Self {
            addr,
            compensator: None,
        }
    }
}

pub struct BME280<I2C> {
    i2c: I2C,
    state: BME280State,
}

impl<I2C> BME280<I2C> {
    pub fn new(i2c: I2C, state: BME280State) -> Self {
        Self { i2c, state }
    }

    pub fn release(self) -> (I2C, BME280State) {
        (self.i2c, self.state)
    }
}

impl<I2C> BME280<I2C>
where
    I2C: Write + WriteRead,
{
    fn write_reg(&mut self, reg: Register, value: u8) -> Result<(), Error> {
        let buf = [reg as u8, value];
        self.i2c
            .write(self.state.addr, &buf)
            .map_err(|_| Error::BusWrite)
    }

    fn read_regs(&mut self, regs: &[Register], dst: &mut [u8]) -> Result<(), Error> {
        for (i, &reg) in regs.iter().enumerate() {
            self.i2c
                .write_read(self.state.addr, &[reg as u8], &mut dst[i..i + 1])
                .map_err(|_| Error::BusRead)?;
        }

        Ok(())
        // TODO: Why this does not work?
        // self.i2c
        //     .write_read(
        //         self.addr,
        //         // SAFETY: Safe because Register has repr(u8)
        //         unsafe { core::mem::transmute(regs) },
        //         dst,
        //     )
        //     .map_err(|_| Error::BusRead)
    }

    pub fn init(&mut self) -> Result<(), Error> {
        let mut chip_id = [0u8];
        self.read_regs(&[Register::ChipId], &mut chip_id)?;
        if chip_id[0] != 0x60 {
            return Err(Error::WrongChipId);
        }

        self.set_settings()?;
        self.calibrate()
    }

    fn set_settings(&mut self) -> Result<(), Error> {
        const HUMIDITY_OVERSAMPLING: u8 = 7;
        self.write_reg(Register::CtrlHum, HUMIDITY_OVERSAMPLING)?;

        const TEMP_OVERSAMPLING: u8 = 1;
        const PRESSURE_OVERSAMPLING: u8 = 1;
        const SENSOR_MODE: u8 = 3; // normal mode
        self.write_reg(
            Register::CtrlMeas,
            (TEMP_OVERSAMPLING << 5) | (PRESSURE_OVERSAMPLING << 2) | SENSOR_MODE,
        )?;

        const STANDBY: u8 = 5; // 1000ms
        const FILTER: u8 = 0; // off
        const SPI_ENABLE: u8 = 0; // disable
        self.write_reg(
            Register::Config,
            (STANDBY << 5) | (FILTER << 2) | SPI_ENABLE,
        )?;

        Ok(())
    }

    fn calibrate(&mut self) -> Result<(), Error> {
        use Register::*;

        let t_regs = [DigT1LSB, DigT1MSB, DigT2LSB, DigT2MSB, DigT3LSB, DigT3MSB];
        let mut t_bytes = [0u8; 6];
        self.read_regs(&t_regs, &mut t_bytes)?;

        let p_regs = [
            DigP1LSB, DigP1MSB, DigP2LSB, DigP2MSB, DigP3LSB, DigP3MSB, DigP4LSB, DigP4MSB,
            DigP5LSB, DigP5MSB, DigP6LSB, DigP6MSB, DigP7LSB, DigP7MSB, DigP8LSB, DigP8MSB,
            DigP9LSB, DigP9MSB,
        ];
        let mut p_bytes = [0u8; 18];
        self.read_regs(&p_regs, &mut p_bytes)?;

        let h_regs = [
            DigH1,
            DigH2LSB,
            DigH2MSB,
            DigH3,
            DigH4MSB,
            DigH4LSBDigH5MSB,
            DigH5LSB,
            DigH6,
        ];
        let mut h_bytes = [0u8; 8];
        self.read_regs(&h_regs, &mut h_bytes)?;

        let compensator = ADCCompensator {
            digt1: u16::from_le_bytes(t_bytes[0..2].try_into().unwrap()),
            digt2: i16::from_le_bytes(t_bytes[2..4].try_into().unwrap()),
            digt3: i16::from_le_bytes(t_bytes[4..6].try_into().unwrap()),

            digp1: u16::from_le_bytes(p_bytes[0..2].try_into().unwrap()),
            digp2: i16::from_le_bytes(p_bytes[2..4].try_into().unwrap()),
            digp3: i16::from_le_bytes(p_bytes[4..6].try_into().unwrap()),
            digp4: i16::from_le_bytes(p_bytes[6..8].try_into().unwrap()),
            digp5: i16::from_le_bytes(p_bytes[8..10].try_into().unwrap()),
            digp6: i16::from_le_bytes(p_bytes[10..12].try_into().unwrap()),
            digp7: i16::from_le_bytes(p_bytes[12..14].try_into().unwrap()),
            digp8: i16::from_le_bytes(p_bytes[14..16].try_into().unwrap()),
            digp9: i16::from_le_bytes(p_bytes[16..18].try_into().unwrap()),

            digh1: h_bytes[0],
            digh2: i16::from_le_bytes(h_bytes[1..3].try_into().unwrap()),
            digh3: h_bytes[3],
            digh4: ((h_bytes[4] as i16) << 4) | (h_bytes[5] & 0x0F) as i16,
            digh5: ((h_bytes[6] as i16) << 4) | (((h_bytes[5] >> 4) & 0x0F) as i16),
            digh6: h_bytes[7] as i8,
        };
        self.state.compensator.replace(compensator);

        Ok(())
    }

    pub fn read_params(&mut self) -> Result<(Temperature, Pressure, Humidity), Error> {
        use Register::*;
        let regs = [
            PressMSB, PressLSB, PressXLSB, TempMSB, TempLSB, TempXLSB, HumMSB, HumLSB,
        ];
        let mut bytes = [0u8; 8];
        self.read_regs(&regs, &mut bytes)?;

        let Some(compensator) = self.state.compensator.borrow_mut() else {
            return Err(Error::NotInitialized);
        };

        let p = ((bytes[0] as i32) << 12) | ((bytes[1] as i32) << 4) | ((bytes[2] as i32) >> 4);
        let t = ((bytes[3] as i32) << 12) | ((bytes[4] as i32) << 4) | ((bytes[5] as i32) >> 4);
        let h = ((bytes[6] as i32) << 8) | (bytes[7] as i32);
        // TODO: humidity returns bogus values, the error is most likely in
        // calibration/reading adc value

        let (t, p, h) = compensator.compensate_tph(t, p, h);
        Ok((
            Temperature::from_raw(t),
            Pressure::from_raw(p),
            Humidity::from_raw(h),
        ))
    }
}

#[derive(Default, Debug)]
struct ADCCompensator {
    // Temperature compensation
    digt1: u16,
    digt2: i16,
    digt3: i16,
    // Pressure compensation
    digp1: u16,
    digp2: i16,
    digp3: i16,
    digp4: i16,
    digp5: i16,
    digp6: i16,
    digp7: i16,
    digp8: i16,
    digp9: i16,
    // Humidity compensation
    digh1: u8,
    digh2: i16,
    digh3: u8,
    digh4: i16,
    digh5: i16,
    digh6: i8,
}

impl ADCCompensator {
    fn compensate_tph(&mut self, t: i32, p: i32, h: i32) -> (i32, u32, u32) {
        let (t, t_fine) = self.compensate_t(t);
        let p = self.compensate_p(p, t_fine);
        let h = self.compensate_h(h, t_fine);
        (t, p, h)
    }

    fn compensate_t(&mut self, adc_t: i32) -> (i32, i32) {
        let var1 = (((adc_t >> 3) - ((self.digt1 as i32) << 1)) * (self.digt2 as i32)) >> 11;
        let var2 =
            (((((adc_t >> 4) - (self.digt1 as i32)) * ((adc_t >> 4) - (self.digt1 as i32))) >> 12)
                * (self.digt3 as i32))
                >> 14;

        let t_fine = var1 + var2;
        ((t_fine * 5 + 128) >> 8, t_fine)
    }

    fn compensate_p(&self, adc_p: i32, t_fine: i32) -> u32 {
        let a = t_fine as i64 - 128000;
        let b = a * a * self.digp6 as i64;
        let b = b + ((a * self.digp5 as i64) << 17);
        let b = b + ((self.digp4 as i64) << 35);
        let a = ((a * a * (self.digp3 as i64)) >> 8) + ((a * (self.digp2 as i64)) << 12);
        let a = ((1 << 47) + a) * (self.digp1 as i64) >> 33;
        if a == 0 {
            return 0;
        }

        let p = 1048576 - adc_p as i64;
        let p = (((p << 31) - b) * 3125) / a;
        let a = ((self.digp9 as i64) * (p >> 13) * (p >> 13)) >> 25;
        let b = ((self.digp8 as i64) * p) >> 19;
        let p = ((p + a + b) >> 8) + ((self.digp7 as i64) << 4);

        p as u32
    }

    fn compensate_h(&self, adc_h: i32, t_fine: i32) -> u32 {
        let v_x1_u32r = t_fine - 76800;
        let v_x1_u32r =
            ((((adc_h << 14) - ((self.digh4 as i32) << 20) - ((self.digh5 as i32) * v_x1_u32r))
                + 16384)
                >> 15)
                * (((((((v_x1_u32r * (self.digh6 as i32)) >> 10)
                    * (((v_x1_u32r * (self.digh3 as i32)) >> 11) + 32768))
                    >> 10)
                    + 2097152)
                    * (self.digh2 as i32)
                    + 8192)
                    >> 14);

        let v_x1_u32r =
            v_x1_u32r - ((((v_x1_u32r >> 15) * (v_x1_u32r >> 15)) >> 7) * (self.digh1 as i32)) >> 4;
        let v_x1_u32r = if v_x1_u32r < 0 { 0 } else { v_x1_u32r };
        let v_x1_u32r = if v_x1_u32r > 419430400 {
            419430400
        } else {
            v_x1_u32r
        };
        (v_x1_u32r >> 12) as u32
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
enum Register {
    DigT1LSB = 0x88,
    DigT1MSB = 0x89,
    DigT2LSB = 0x8A,
    DigT2MSB = 0x8B,
    DigT3LSB = 0x8C,
    DigT3MSB = 0x8D,
    DigP1LSB = 0x8E,
    DigP1MSB = 0x8F,
    DigP2LSB = 0x90,
    DigP2MSB = 0x91,
    DigP3LSB = 0x92,
    DigP3MSB = 0x93,
    DigP4LSB = 0x94,
    DigP4MSB = 0x95,
    DigP5LSB = 0x96,
    DigP5MSB = 0x97,
    DigP6LSB = 0x98,
    DigP6MSB = 0x99,
    DigP7LSB = 0x9A,
    DigP7MSB = 0x9B,
    DigP8LSB = 0x9C,
    DigP8MSB = 0x9D,
    DigP9LSB = 0x9E,
    DigP9MSB = 0x9F,

    DigH1 = 0xA1,
    DigH2MSB = 0xE1,
    DigH2LSB = 0xE2,
    DigH3 = 0xE3,
    DigH4MSB = 0xE4,
    DigH4LSBDigH5MSB = 0xE5,
    DigH5LSB = 0xE6,
    DigH6 = 0xE7,

    PressMSB = 0xF7,
    PressLSB = 0xF8,
    PressXLSB = 0xF9,
    TempMSB = 0xFA,
    TempLSB = 0xFB,
    TempXLSB = 0xFC,
    HumMSB = 0xFD,
    HumLSB = 0xFE,

    CtrlHum = 0xF2,
    CtrlMeas = 0xF4,
    Config = 0xF5,

    ChipId = 0xD0,
}

#[derive(Debug)]
pub enum Error {
    BusRead,
    BusWrite,
    WrongChipId,
}
