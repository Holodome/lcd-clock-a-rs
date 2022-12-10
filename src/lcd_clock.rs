//! General project-wide functionality
use core::borrow::Borrow;

use crate::{
    drivers::{
        bme280, ds3231,
        ds3231::{Date, Time},
        st7789vwx6,
        st7789vwx6::Display,
    },
    hardware::LcdClockHardware,
    images::{MENUPIC_A, NUMPIC_A},
    led_strip::{LedMode, LED_COUNT},
    misc::{ColorRGB565, ColorRGB8, Sin},
    state::{AppMode, MenuOption, State, TimeDateScreen},
};

pub struct LcdClock {
    hardware: LcdClockHardware,
    state: State,

    /// Used as comparator value needed to decide which displays we want to
    /// update
    last_time: Time,
    last_date: Date,
}

impl LcdClock {
    pub fn new(hardware: LcdClockHardware, sin: Sin) -> Self {
        Self {
            hardware,
            state: State::new(sin),
            last_time: Default::default(),
            last_date: Default::default(),
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.hardware.init()?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), Error> {
        self.update_buttons();

        let transition = self.state.eat_transition();
        match self.state.mode() {
            AppMode::Regular(screen) => match screen {
                TimeDateScreen::Time => {
                    self.mode_time(transition)?;
                }
                TimeDateScreen::Date => {
                    self.mode_date(transition)?;
                }
            },
            AppMode::Menu(menu) => self.mode_menu(menu, transition)?,
            AppMode::SetRgb => self.mode_rgb(transition)?,
            _ => {}
        }

        // TODO: dynamic update time (using rtc or system timer)
        cortex_m::asm::delay(125 * 1000 * 16);
        self.state.update();
        self.hardware
            .led_strip
            .display(self.state.led_strip().colors());

        Ok(())
    }

    fn mode_menu(&mut self, selected_mode: MenuOption, force_update: bool) -> Result<(), Error> {
        if !force_update {
            return Ok(());
        }

        for (mode, display) in MenuOption::all().zip(Display::all()) {
            let pic = MENUPIC_A.get_pic(mode);
            self.hardware.with_gl(|gl| gl.draw_pic(display, pic))?;

            if mode == selected_mode {
                let w = self.hardware.displays.width();
                let h = self.hardware.displays.height();
                let thickness = 8;
                let color = ColorRGB565::from(ColorRGB8::red());
                self.hardware.with_gl(|gl| {
                    gl.draw_rect(display, 0, 0, w, thickness, color)?;
                    gl.draw_rect(display, 0, thickness, thickness, h, color)?;
                    gl.draw_rect(display, w - thickness, thickness, w, h, color)?;
                    gl.draw_rect(display, thickness, h - thickness, w - thickness, h, color)
                })?;
            }
        }

        Ok(())
    }

    fn mode_time(&mut self, force_update: bool) -> Result<(), Error> {
        let time = self
            .hardware
            .with_rtc(|rtc| rtc.get_time())?
            .map_err(Error::Rtc)?;

        let time_displays = time_to_display_values(time);
        let prev_time_displays = time_to_display_values(self.last_time);

        for ((display, &time), &prev) in Display::all()
            .into_iter()
            .zip(time_displays.iter())
            .zip(prev_time_displays.iter())
        {
            if let Some(pic) = NUMPIC_A.get_digit(time) {
                if time != prev || force_update {
                    self.hardware.with_gl(|gl| gl.draw_pic(display, pic))?;
                }
            }
        }

        self.last_time = time;

        Ok(())
    }

    fn mode_date(&mut self, force_update: bool) -> Result<(), Error> {
        let date = self
            .hardware
            .with_rtc(|rtc| rtc.get_calendar())?
            .map_err(Error::Rtc)?;

        let date_displays = date_to_display_values(date);
        let prev_date_displays = date_to_display_values(self.last_date);
        for ((display, &cur), &prev) in Display::all()
            .into_iter()
            .zip(date_displays.iter())
            .zip(prev_date_displays.iter())
        {
            if cur != prev || force_update {
                if let Some(pic) = NUMPIC_A.get_digit(cur) {
                    self.hardware.with_gl(|gl| gl.draw_pic(display, pic))?;
                }
            }
        }

        self.last_date = date;

        Ok(())
    }

    fn mode_rgb(&mut self, force_update: bool) -> Result<(), Error> {
        let colors = match self.state.led_strip().mode() {
            LedMode::Sin => [
                ColorRGB8::red(),
                ColorRGB8::green(),
                ColorRGB8::blue(),
                ColorRGB8::cyan(),
                ColorRGB8::yellow(),
                ColorRGB8::pink(),
            ],
            LedMode::Off => [ColorRGB8::black(); LED_COUNT],
            LedMode::Red => [ColorRGB8::red(); LED_COUNT],
            LedMode::Green => [ColorRGB8::green(); LED_COUNT],
            LedMode::Blue => [ColorRGB8::blue(); LED_COUNT],
            LedMode::Cyan => [ColorRGB8::cyan(); LED_COUNT],
            LedMode::Yellow => [ColorRGB8::yellow(); LED_COUNT],
            LedMode::Pink => [ColorRGB8::pink(); LED_COUNT],
        };

        if force_update {
            for (display, color) in Display::all().zip(colors) {
                self.hardware.with_gl(|gl| gl.fill(display, color.into()))?;
            }
        }

        Ok(())
    }

    fn update_buttons(&mut self) {
        let (mode_button_transition, left_button_transition, right_button_transition) =
            self.hardware.update_buttons();
        self.state.handle_buttons(
            mode_button_transition,
            left_button_transition,
            right_button_transition,
        );
    }
}

#[derive(Debug)]
pub enum Error {
    Display(st7789vwx6::Error),
    HumiditySensor(bme280::Error),
    Rtc(ds3231::Error),

    I2CClaim,
}

fn time_to_display_values(time: Time) -> [u8; 6] {
    let houra = time.hours / 10;
    let hourb = time.hours % 10;
    let mina = time.mins / 10;
    let minb = time.mins % 10;
    let seca = time.secs / 10;
    let secb = time.secs % 10;

    [houra, hourb, mina, minb, seca, secb]
}

fn date_to_display_values(date: Date) -> [u8; 6] {
    let yeara = (date.year % 100) / 10;
    let yearb = date.year % 10;
    let montha = date.month / 10;
    let monthb = date.month % 10;
    let datea = date.date / 10;
    let dateb = date.date % 10;

    [yeara as u8, yearb as u8, montha, monthb, datea, dateb]
}
