//! General project-wide functionality

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

/// Main application. Its functionality loosely corresponds to View in MVC.
pub struct LcdClock {
    hardware: LcdClockHardware,
    state: State,

    /// Used as comparator value needed to decide which displays we want to
    /// update
    last_time: Time,
    last_date: Date,
    last_brightness: u32,
}

impl LcdClock {
    pub fn new(hardware: LcdClockHardware, sin: Sin, brightness: u32) -> Self {
        let state = State::new(sin, brightness);
        let last_brightness = brightness;
        Self {
            hardware,
            state,
            last_time: Default::default(),
            last_date: Default::default(),
            last_brightness,
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.hardware.init()?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), Error> {
        self.update_buttons();

        let brightness = self.state.brightness();
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
            AppMode::SetTime(screen_index) => self.mode_set_time(screen_index, transition)?,
            AppMode::SetAlarm(screen_index) => self.mode_set_time(screen_index, transition)?,
            AppMode::SetRgb => self.mode_rgb(transition)?,
            AppMode::SetBrightness => self.mode_brightness(transition, brightness)?,
            _ => {}
        }

        if let Some(time_delta) = self.state.take_time_delta() {
            let (index, change) = time_delta;
            if matches!(self.state.mode(), AppMode::SetTime(..)) {
                self.change_time(index, change)?;
            } else {
                // self.change_alarm(index, change)?;
            }
        }

        if brightness != self.last_brightness {
            self.last_brightness = brightness;
            let brightness_mapped = (u16::MAX / 10) * brightness as u16;
            self.hardware.displays.set_brightness(brightness_mapped);
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

        let last_mode = self.state.last_mode();
        let last_mode = match last_mode {
            AppMode::Menu(menu) => Some(menu),
            _ => None,
        };

        for (mode, display) in MenuOption::all().zip(Display::all()) {
            // avoid redrawing screens that did not change
            if let Some(last_mode) = last_mode {
                if last_mode != mode && mode != selected_mode {
                    continue;
                }
            }

            let pic = MENUPIC_A.get_pic(mode);
            self.hardware.with_gl(|gl| gl.draw_pic(display, pic))?;

            if mode == selected_mode {
                let thickness = 8;
                let color = ColorRGB565::from(ColorRGB8::red());
                self.hardware
                    .with_gl(|gl| gl.draw_bounding_rect(display, thickness, color))?;
            }
        }

        Ok(())
    }

    fn mode_set_time(&mut self, screen_index: usize, force_update: bool) -> Result<(), Error> {
        // here we don't save time by not redrawing all displays because settings time
        // is such unfrequent operation that we practically don't care
        if screen_index < 6 {
            self.mode_time(force_update)?;
        } else {
            self.mode_date(force_update)?;
        }

        let display = match screen_index % 6 {
            0 => Display::D1,
            1 => Display::D2,
            2 => Display::D3,
            3 => Display::D4,
            4 => Display::D5,
            5 => Display::D6,
            _ => Display::D1,
        };
        let thickness = 8;
        let color = ColorRGB565::from(ColorRGB8::red());
        self.hardware
            .with_gl(|gl| gl.draw_bounding_rect(display, thickness, color))?;

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

    fn mode_brightness(&mut self, force_update: bool, brightness: u32) -> Result<(), Error> {
        if force_update {
            for display in Display::all() {
                if let Some(pic) = NUMPIC_A.get_digit(brightness as u8) {
                    self.hardware.with_gl(|gl| gl.draw_pic(display, pic))?;
                }
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

    fn change_time(&mut self, index: usize, change: i8) -> Result<(), Error> {
        if index < 6 {
            let time = self
                .hardware
                .with_rtc(|rtc| rtc.get_time())?
                .map_err(Error::Rtc)?;
            let mut new_time = time;
            match index {
                0 => new_time.hours = time.hours.saturating_add_signed(change * 10),
                1 => new_time.hours = time.hours.saturating_add_signed(change * 1),
                2 => new_time.mins = time.mins.saturating_add_signed(change * 10),
                3 => new_time.mins = time.mins.saturating_add_signed(change * 1),
                4 => new_time.secs = time.secs.saturating_add_signed(change * 10),
                5 => new_time.secs = time.secs.saturating_add_signed(change * 1),
                _ => {}
            }
            new_time.hours %= 24;
            new_time.mins %= 60;
            new_time.secs %= 60;
            if new_time.hours != time.hours {
                self.hardware
                    .with_rtc(|rtc| rtc.set_hours(new_time.hours))?
                    .map_err(Error::Rtc)?;
            } else if new_time.mins != time.mins {
                self.hardware
                    .with_rtc(|rtc| rtc.set_mins(new_time.mins))?
                    .map_err(Error::Rtc)?;
            } else {
                self.hardware
                    .with_rtc(|rtc| rtc.set_secs(new_time.secs))?
                    .map_err(Error::Rtc)?;
            }
        } else {
            let date = self
                .hardware
                .with_rtc(|rtc| rtc.get_calendar())?
                .map_err(Error::Rtc)?;
            let mut new_date = date;
            match index % 6 {
                0 => new_date.year = date.year.saturating_add_signed(change as i16 * 10),
                1 => new_date.year = date.year.saturating_add_signed(change as i16 * 1),
                2 => new_date.month = date.month.saturating_add_signed(change * 10),
                3 => new_date.month = date.month.saturating_add_signed(change * 1),
                4 => new_date.date = date.date.saturating_add_signed(change * 10),
                5 => new_date.date = date.date.saturating_add_signed(change * 1),
                _ => {}
            }
            if new_date.year != date.year {
                self.hardware
                    .with_rtc(|rtc| rtc.set_year(new_date.year))?
                    .ok();
            } else if new_date.month != date.month {
                self.hardware
                    .with_rtc(|rtc| rtc.set_month(new_date.month))?
                    .ok();
            } else {
                self.hardware
                    .with_rtc(|rtc| rtc.set_date(new_date.date))?
                    .ok();
            }
        }

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
