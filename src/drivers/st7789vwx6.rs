//! Driver fot ST7789VW
//!
//! LCD-Nixie-Clock has 6 135x240 displays. They all use SPI interface on a
//! single line and use 3 CS (chip select) pins to select display per draw
//! command.
//!
//! The system has some hardware quirks that we have to consider - thus the
//! implementation of display driver is not generic. Firstly, there are some
//! hardware settings that had to be copied from sample code provided by
//! waveshare. They regard voltage and gamma - things that are usually not
//! covered in generic drivers. Secondly, screen locations have to be offsetted.
//! Thirdly, there are 3 CS lines - instead of usual for these kind of displays
//! 1. Waveshare most probably placed a binary decoder circuit that transforms
//! display number set on 3 CS lines into CS for each display independently.
//!
//! Another addition is a pin that controls brightness of displays. We can
//! attach it to PWM and set brightness dynamically.
use core::convert::Infallible;
use embedded_hal::{
    blocking::spi::Write,
    digital::v2::{OutputPin, PinState},
    PwmPin,
};
use unwrap_infallible::UnwrapInfallible;

pub const WIDTH: u16 = 135;
pub const HEIGHT: u16 = 240;

/// One of the six displays left-to-right.
/// These are identical and are driven by 3 CS lines.
#[derive(Debug, Clone, Copy)]
pub enum Display {
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
}

impl Display {
    /// Order of displays is inversed
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

    pub fn all() -> impl Iterator<Item = Self> {
        [
            Display::D1,
            Display::D2,
            Display::D3,
            Display::D4,
            Display::D5,
            Display::D6,
        ]
        .iter()
        .copied()
    }
}

/// Driver for 6 ST7789VW displays.
pub struct ST7789VWx6<PINS, SPI, BL> {
    pins: PINS,
    spi: SPI,
    bl: BL,

    width: u16,
    height: u16,
    brightness: u16,
}

impl<PINS, SPI, BL> ST7789VWx6<PINS, SPI, BL> {
    pub fn new(pins: PINS, spi: SPI, bl: BL, width: u16, height: u16, brightness: u16) -> Self {
        Self {
            pins,
            spi,
            bl,
            width,
            height,
            brightness,
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }
}

impl<PINS, SPI, BL> ST7789VWx6<PINS, SPI, BL>
where
    PINS: Pins,
    SPI: Write<u8>,
    BL: PwmPin<Duty = u16>,
{
    pub fn set_brightness(&mut self, brightness: u16) {
        self.brightness = brightness;
        self.bl.set_duty(self.brightness);
    }

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
        f: impl FnOnce(&mut ST7789VWx6<PINS, SPI, BL>) -> Res,
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
        x_end += 52 - 1;
        y_start += 40;
        y_end += 40 - 1;
        self.send_command(Command::CASET)?;
        let mut x = [0u8; 4];
        x[0..2].copy_from_slice(&x_start.to_be_bytes());
        x[2..4].copy_from_slice(&x_end.to_be_bytes());
        self.send_data(&x)?;
        self.send_command(Command::RASET)?;
        let mut y = [0u8; 4];
        y[0..2].copy_from_slice(&y_start.to_be_bytes());
        y[2..4].copy_from_slice(&y_end.to_be_bytes());
        self.send_data(&y)?;

        Ok(())
    }

    fn hard_reset(&mut self) {
        self.pins.rst().set_high().unwrap_infallible();
        // reset for at least 10 us as specified in datasheet.
        // max rp2040 clock frequency is 133 mhz, so we need to sleep for at
        // least 133 * 10 cycles.
        cortex_m::asm::delay(125 * 10);
        self.pins.rst().set_low().unwrap_infallible();
        cortex_m::asm::delay(125 * 10);
        self.pins.rst().set_high().unwrap_infallible();
        cortex_m::asm::delay(125 * 10);
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

    pub fn init(&mut self) -> Result<(), Error> {
        self.hard_reset();
        self.set_brightness(self.brightness);

        for display in Display::all() {
            self.with_cs(display, Self::init_display)?;
        }

        Ok(())
    }

    pub fn set_pixels(
        &mut self,
        display: Display,
        x_start: u16,
        y_start: u16,
        x_end: u16,
        y_end: u16,
        colors: &[u8],
    ) -> Result<(), Error> {
        self.with_cs(display, |d| {
            d.set_region(x_start, y_start, x_end, y_end)?;
            d.send_command(Command::RAMWR)?;
            d.send_data(colors)?;

            Ok(())
        })
    }

    pub fn set_pixels_iter<T>(
        &mut self,
        display: Display,
        x_start: u16,
        y_start: u16,
        x_end: u16,
        y_end: u16,
        colors: T,
    ) -> Result<(), Error>
    where
        T: IntoIterator<Item = u8>,
    {
        self.with_cs(display, |d| {
            d.set_region(x_start, y_start, x_end, y_end)?;
            d.send_command(Command::RAMWR)?;

            let mut buf = [0u8; 256];
            let mut i = 0;

            for v in colors.into_iter() {
                buf[i] = v;
                i += 1;

                if i == buf.len() {
                    d.send_data(&buf)?;
                    i = 0;
                }
            }

            if i != 0 {
                d.send_data(&buf)?;
            }

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

    fn dc(&mut self) -> &mut DC {
        &mut self.3
    }

    fn rst(&mut self) -> &mut RST {
        &mut self.4
    }
}

#[derive(Debug)]
pub enum Error {
    OutOfBounds,
    BusWrite,
}

#[allow(clippy::upper_case_acronyms)]
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
