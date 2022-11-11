use core::convert::Infallible;
use embedded_hal::{
    blocking::{delay::DelayUs, spi::Write},
    digital::v2::{OutputPin, PinState},
};
use unwrap_infallible::UnwrapInfallible;

#[derive(Clone, Copy)]
pub enum Display {
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
}

impl Display {
    fn into_cs_value(self) -> usize {
        match self {
            Self::D1 => 5,
            Self::D2 => 4,
            Self::D3 => 3,
            Self::D4 => 2,
            Self::D5 => 1,
            Self::D6 => 0,
        }
    }

    fn into_cs_states(self) -> (PinState, PinState, PinState) {
        let value = self.into_cs_value();
        (
            (value & 0x1 != 0).into(),
            (value & 0x2 != 0).into(),
            (value & 0x4 != 0).into(),
        )
    }
}

pub struct ST7789VWx6<PINS, SPI> {
    pins: PINS,
    spi: SPI,
}

impl<PINS, SPI> ST7789VWx6<PINS, SPI> {
    pub fn new(pins: PINS, spi: SPI) -> Self {
        Self { pins, spi }
    }
}

impl<PINS, SPI> ST7789VWx6<PINS, SPI>
where
    PINS: Pins,
    SPI: Write<u8>,
{
    fn cs_low(&mut self, display: Display) {
        let states = display.into_cs_states();
        self.pins.csa1().set_state(states.0).unwrap_infallible();
        self.pins.csa2().set_state(states.1).unwrap_infallible();
        self.pins.csa3().set_state(states.2).unwrap_infallible();
    }

    fn cs_high(&mut self) {
        self.pins.csa1().set_high().unwrap_infallible();
        self.pins.csa2().set_high().unwrap_infallible();
        self.pins.csa3().set_high().unwrap_infallible();
    }

    fn with_cs<Res>(
        &mut self,
        display: Display,
        f: impl FnOnce(&mut ST7789VWx6<PINS, SPI>) -> Res,
    ) -> Res {
        self.cs_low(display);
        let result = f(self);
        self.cs_high();

        result
    }

    fn send_commands(&mut self, cmds: &[u8]) -> Result<(), Error> {
        self.pins.dc().set_low().unwrap_infallible();
        self.spi.write(cmds).map_err(|_| Error::BusWrite)
    }

    fn send_data(&mut self, data: &[u8]) -> Result<(), Error> {
        self.pins.dc().set_high().unwrap_infallible();
        self.spi.write(data).map_err(|_| Error::BusWrite)
    }

    fn set_region(
        &mut self,
        x_start: u16,
        y_start: u16,
        x_end: u16,
        y_end: u16,
    ) -> Result<(), Error> {
        self.send_commands(&[Command::CASET as u8])?;
        let mut x = [0u8; 4];
        x[0..2].copy_from_slice(&x_start.to_le_bytes());
        x[2..4].copy_from_slice(&x_end.to_le_bytes());
        self.send_data(&x)?;
        self.send_commands(&[Command::RASET as u8])?;
        let mut y = [0u8; 4];
        y[0..2].copy_from_slice(&y_start.to_le_bytes());
        y[2..4].copy_from_slice(&y_end.to_le_bytes());
        self.send_data(&y)?;

        Ok(())
    }

    fn hard_reset<DELAY: DelayUs<u32>>(&mut self, delay: &mut DELAY) {
        self.pins.rst().set_high().unwrap_infallible();
        delay.delay_us(10);
        self.pins.rst().set_low().unwrap_infallible();
        delay.delay_us(10);
        self.pins.rst().set_high().unwrap_infallible();
        delay.delay_us(10);
    }

    pub fn init<DELAY: DelayUs<u32>>(&mut self, delay: &mut DELAY) -> Result<(), Error> {
        self.hard_reset(delay);

        // refresh from left to right, bottom from to top, use rgb
        self.send_commands(&[Command::MADCTL as u8])?;
        self.send_data(&[0b0000_0000])?;
        // 65k 16 bits/pixel colors
        self.send_commands(&[Command::COLMOD as u8])?;
        self.send_data(&[0b0101_0101])?;
        // have no idea what it does...
        self.send_commands(&[Command::PORCTRL as u8])?;
        self.send_data(&[0x0C, 0x0C, 0x00, 0x33, 0x33])?;
        // ...
        self.send_commands(&[Command::GCTRL as u8])?;
        self.send_data(&[0x35])?;

        Ok(())
    }

    pub fn set_pixel(&mut self, display: Display, x: u16, y: u16, color: u16) -> Result<(), Error> {
        self.with_cs(display, |d| {
            d.set_region(x, y, x, y)?;
            d.send_commands(&[Command::RAMWR as u8])?;
            d.send_data(&color.to_be_bytes())
        })
    }
}

pub trait Pins {
    type CSA1: OutputPin<Error = Infallible>;
    type CSA2: OutputPin<Error = Infallible>;
    type CSA3: OutputPin<Error = Infallible>;
    type DC: OutputPin<Error = Infallible>;
    type RST: OutputPin<Error = Infallible>;

    fn csa1(&mut self) -> &mut Self::CSA1;
    fn csa2(&mut self) -> &mut Self::CSA2;
    fn csa3(&mut self) -> &mut Self::CSA3;
    fn dc(&mut self) -> &mut Self::DC;
    fn rst(&mut self) -> &mut Self::RST;
}

impl<
        CSA1: OutputPin<Error = Infallible>,
        CSA2: OutputPin<Error = Infallible>,
        CSA3: OutputPin<Error = Infallible>,
        DC: OutputPin<Error = Infallible>,
        RST: OutputPin<Error = Infallible>,
    > Pins for (CSA1, CSA2, CSA3, DC, RST)
{
    type CSA1 = CSA1;
    type CSA2 = CSA2;
    type CSA3 = CSA3;
    type DC = DC;
    type RST = RST;

    fn csa1(&mut self) -> &mut CSA1 {
        &mut self.0
    }

    fn csa2(&mut self) -> &mut CSA2 {
        &mut self.1
    }

    fn csa3(&mut self) -> &mut CSA3 {
        &mut self.2
    }

    fn dc(&mut self) -> &mut Self::DC {
        &mut self.3
    }

    fn rst(&mut self) -> &mut Self::RST {
        &mut self.4
    }
}

pub enum Error {
    OutOfBounds,
    BusWrite,
}

enum Command {
    /// Memory data access control
    MADCTL = 0x36,
    /// Interface pixel format
    COLMOD = 0x3A,
    /// Porch setting
    PORCTRL = 0xB2,
    /// Gate control
    GCTRL = 0xb7,
    /// Column address set
    CASET = 0x2A,
    /// Row address set
    RASET = 0x2B,
    /// Memory write
    RAMWR = 0x2C,
}
