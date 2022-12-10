use crate::{drivers::buttons::ButtonEvent, led_strip::LedStripState, misc::Sin};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum TimeDateScreen {
    #[default]
    Time,
    Date,
}

impl TimeDateScreen {
    fn left(self) -> Self {
        match self {
            Self::Time => Self::Date,
            Self::Date => Self::Time,
        }
    }

    pub fn right(self) -> Self {
        match self {
            Self::Time => Self::Date,
            Self::Date => Self::Time,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuOption {
    /// Set time and date
    SetTime,
    /// Set alarm settings
    SetAlarm,
    /// Change behaviour of backlight
    SetRgb,
    /// Set brightness of display
    SetBrightness,
    /// View temperature, humidity and pressure
    TempHumidity,
    /// Return back to regular mode
    Return,
}

impl MenuOption {
    pub fn left(self) -> Self {
        match self {
            Self::SetTime => Self::Return,
            Self::SetAlarm => Self::SetTime,
            Self::SetRgb => Self::SetAlarm,
            Self::SetBrightness => Self::SetRgb,
            Self::TempHumidity => Self::SetBrightness,
            Self::Return => Self::TempHumidity,
        }
    }

    pub fn right(self) -> Self {
        match self {
            Self::SetTime => Self::SetAlarm,
            Self::SetAlarm => Self::SetRgb,
            Self::SetRgb => Self::SetBrightness,
            Self::SetBrightness => Self::TempHumidity,
            Self::TempHumidity => Self::Return,
            Self::Return => Self::SetTime,
        }
    }

    pub fn all() -> impl Iterator<Item = Self> {
        [
            Self::SetTime,
            Self::SetAlarm,
            Self::SetRgb,
            Self::SetBrightness,
            Self::TempHumidity,
            Self::Return,
        ]
        .iter()
        .copied()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppMode {
    Regular(TimeDateScreen),
    Menu(MenuOption),
    SetTime(TimeDateScreen),
    SetAlarm(TimeDateScreen),
    SetRgb,
    SetBrightness,
    TempHumidity,
}

/// State of application. It tries to store all things that may change based
/// on user input and modify it in a single place.
pub struct State {
    /// FSM for application mode. It basically enumerates all possible screens.
    mode: AppMode,
    /// Led strip has state on its own in order to create animations
    led_strip: LedStripState,
    /// Has state transition occured? Application can use this information in
    /// order to decide whether to redraw or not.
    transition: bool,
    /// Is mode button down?
    is_mode_down: bool,
}

impl State {
    pub fn new(sin: Sin) -> Self {
        Self {
            mode: AppMode::Regular(Default::default()),
            led_strip: LedStripState::new(sin),
            transition: true,
            is_mode_down: false,
        }
    }

    pub fn led_strip(&self) -> &LedStripState {
        &self.led_strip
    }

    pub fn mode(&self) -> AppMode {
        self.mode
    }

    pub fn eat_transition(&mut self) -> bool {
        let result = self.transition;
        self.transition = false;
        result
    }

    pub fn handle_buttons(
        &mut self,
        mode: Option<ButtonEvent>,
        left: Option<ButtonEvent>,
        right: Option<ButtonEvent>,
    ) {
        match mode {
            Some(ButtonEvent::Release) => self.is_mode_down = false,
            Some(ButtonEvent::Press) => self.is_mode_down = true,
            _ => {}
        }

        let mode = matches!(mode, Some(ButtonEvent::Release));
        let left = matches!(left, Some(ButtonEvent::Release));
        let right = matches!(right, Some(ButtonEvent::Release));
        match self.mode {
            AppMode::Regular(screen) => {
                if mode {
                    self.mode = AppMode::Menu(MenuOption::Return);
                    self.transition = true;
                } else if left {
                    self.mode = AppMode::Regular(screen.left());
                    self.transition = true;
                } else if right {
                    self.mode = AppMode::Regular(screen.right());
                    self.transition = true;
                }
            }
            AppMode::Menu(menu) => {
                if mode {
                    self.mode = match menu {
                        MenuOption::Return => AppMode::Regular(Default::default()),
                        MenuOption::SetTime => AppMode::SetTime(Default::default()),
                        MenuOption::SetAlarm => AppMode::SetAlarm(Default::default()),
                        MenuOption::SetRgb => AppMode::SetRgb,
                        MenuOption::SetBrightness => AppMode::SetBrightness,
                        MenuOption::TempHumidity => AppMode::TempHumidity,
                    };
                    self.transition = true;
                } else if left {
                    self.mode = AppMode::Menu(menu.left());
                    self.transition = true;
                } else if right {
                    self.mode = AppMode::Menu(menu.right());
                    self.transition = true;
                }
            }
            AppMode::SetTime(screen) => {
                if self.is_mode_down {
                    if left {
                        todo!()
                    } else if right {
                        todo!()
                    }
                } else if left {
                    self.mode = AppMode::SetTime(screen.left());
                    self.transition = true;
                } else if right {
                    self.mode = AppMode::SetTime(screen.right());
                    self.transition = true;
                }

                if mode {
                    self.mode = AppMode::Regular(Default::default());
                    self.transition = true;
                }
            }
            AppMode::SetAlarm(screen) => {
                if self.is_mode_down {
                    if left {
                        todo!()
                    } else if right {
                        todo!()
                    }
                } else if left {
                    self.mode = AppMode::SetAlarm(screen.left());
                    self.transition = true;
                } else if right {
                    self.mode = AppMode::SetAlarm(screen.right());
                    self.transition = true;
                }

                if mode {
                    self.mode = AppMode::Regular(Default::default());
                    self.transition = true;
                }
            }
            AppMode::SetRgb => {
                if left {
                    self.led_strip.left();
                    self.transition = true;
                }
                if right {
                    self.led_strip.right();
                    self.transition = true;
                }
            }
            AppMode::SetBrightness => {
                todo!()
            }
            AppMode::TempHumidity => {
                todo!()
            }
        }
    }

    pub fn update(&mut self) {
        self.led_strip.update();
    }
}
