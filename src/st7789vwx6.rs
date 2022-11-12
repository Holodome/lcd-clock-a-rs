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

    fn send_command(&mut self, cmd: Command) -> Result<(), Error> {
        self.send_commands(&[cmd as u8])
    }

    fn send_data(&mut self, data: &[u8]) -> Result<(), Error> {
        self.pins.dc().set_high().unwrap_infallible();
        self.spi.write(data).map_err(|_| Error::BusWrite)
    }

    fn set_region(
        &mut self,
        mut x_start: u16,
        mut y_start: u16,
        mut x_end: u16,
        mut y_end: u16,
    ) -> Result<(), Error> {
        x_start += 52;
        x_end += 52;
        y_start += 40;
        y_end += 40;
        self.send_commands(&[Command::CASET as u8])?;
        let mut x = [0u8; 4];
        x[0..2].copy_from_slice(&x_start.to_be_bytes());
        x[2..4].copy_from_slice(&x_end.to_be_bytes());
        self.send_data(&x)?;
        self.send_commands(&[Command::RASET as u8])?;
        let mut y = [0u8; 4];
        y[0..2].copy_from_slice(&y_start.to_be_bytes());
        y[2..4].copy_from_slice(&y_end.to_be_bytes());
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

    fn init_display(&mut self) -> Result<(), Error> {
        // refresh from left to right, bottom from to top, use rgb
        self.send_command(Command::MADCTL)?;
        self.send_data(&[0b0000_0000])?;
        // 65k 16 bits/pixel colors
        self.send_command(Command::COLMOD)?;
        self.send_data(&[0b0101_0101])?;
        // have no idea what it does...
        self.send_command(Command::PORCTRL)?;
        self.send_data(&[0x0C, 0x0C, 0x00, 0x33, 0x33])?;

        self.send_command(Command::GCTRL)?;
        self.send_data(&[0x35])?;

        self.send_command(Command::VCOMS)?;
        self.send_data(&[0x19])?;

        self.send_command(Command::LCMCTRL)?;
        self.send_data(&[0x2C])?;

        self.send_command(Command::VDVVRHEN)?;
        self.send_data(&[0x01])?;

        self.send_command(Command::VRHS)?;
        self.send_data(&[0x12])?;

        self.send_command(Command::VDVS)?;
        self.send_data(&[0x20])?;

        self.send_command(Command::FRCTRL2)?;
        self.send_data(&[0x0F])?;

        self.send_command(Command::PWCTRL1)?;
        self.send_data(&[0xA4, 0xA1])?;

        self.send_command(Command::PVGAMCTRL)?;
        self.send_data(&[
            0xD0, 0x04, 0x0D, 0x11, 0x13, 0x2B, 0x3F, 0x54, 0x4C, 0x18, 0x0D, 0x0B, 0x1F, 0x23,
        ])?;

        self.send_command(Command::NVGAMCTRL)?;
        self.send_data(&[
            0xD0, 0x04, 0x0C, 0x11, 0x13, 0x2C, 0x3F, 0x44, 0x51, 0x2F, 0x1F, 0x1F, 0x20, 0x23,
        ])?;

        self.send_command(Command::INVON)?;
        // exit sleep mode
        self.send_command(Command::SLPOUT)?;
        // turn on display
        self.send_command(Command::DISPON)?;

        Ok(())
    }

    pub fn init<DELAY: DelayUs<u32>>(&mut self, delay: &mut DELAY) -> Result<(), Error> {
        self.hard_reset(delay);

        // do this for all displays
        for display in [
            Display::D1,
            Display::D2,
            Display::D3,
            Display::D4,
            Display::D5,
            Display::D6,
        ] {
            self.with_cs(display, Self::init_display)?;
        }

        Ok(())
    }

    pub fn set_pixel(&mut self, display: Display, x: u16, y: u16, color: u16) -> Result<(), Error> {
        self.with_cs(display, |d| {
            d.set_region(x, y, x, y)?;
            d.send_commands(&[Command::RAMWR as u8])?;
            d.send_data(&color.to_be_bytes())?;

            Ok(())
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

#[derive(Debug)]
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
    GCTRL = 0xB7,
    /// VCOMS setting
    VCOMS = 0xBB,
    /// LCM Control
    LCMCTRL = 0xC0,
    /// VDV and VRH command enable
    VDVVRHEN = 0xC2,
    /// VRH set
    VRHS = 0xC3,
    /// VDV set
    VDVS = 0xC4,
    /// Frame rate control
    FRCTRL2 = 0xC6,
    /// Power control
    PWCTRL1 = 0xD0,
    /// Positive voltage gamma control
    PVGAMCTRL = 0xE0,
    /// Negative voltage gamma control
    NVGAMCTRL = 0xE1,
    /// Display inversion on
    INVON = 0x21,
    /// Sleep out
    SLPOUT = 0x11,
    /// Display on
    DISPON = 0x29,
    /// Column address set
    CASET = 0x2A,
    /// Row address set
    RASET = 0x2B,
    /// Memory write
    RAMWR = 0x2C,
}
