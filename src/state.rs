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
/// All possible choices in main menu
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
/// All possible application states
pub enum AppMode {
    Regular(TimeDateScreen),
    Menu(MenuOption),
    SetTime(usize),
    SetAlarm(usize),
    SetRgb,
    SetBrightness,
    TempHumidity,
}

/// State of application. It tries to store all things that may change based
/// on user input and modify it in a single place. It loosely corresponds to
/// Controller in MVC.
pub struct State {
    /// Store last mode before changing. Application may use this information in
    /// order to redraw more effeciently.
    last_mode: AppMode,
    /// FSM for application mode. It basically enumerates all possible screens.
    mode: AppMode,
    /// Led strip has state on its own in order to create animations
    led_strip: LedStripState,
    /// Brightness of display (from 0 to 10)
    brightness: u32,
    /// Has state transition occured? Application can use this information in
    /// order to decide whether to redraw or not.
    transition: bool,
    /// Is mode button down?
    is_mode_down: bool,
    /// Flag used to determine behaviour while setting time/alarm. In this cases
    /// if mode was held and either of left or right pressed the time is
    /// changed, otherwise mode button changes mode.
    lr_pressed_while_mode_down: bool,

    time_delta: Option<(usize, i32)>,
}

impl State {
    pub fn new(sin: Sin, brightness: u32) -> Self {
        let mode = AppMode::Regular(Default::default());
        Self {
            mode,
            last_mode: mode,
            led_strip: LedStripState::new(sin),
            brightness,
            transition: true,
            is_mode_down: false,
            lr_pressed_while_mode_down: false,
            time_delta: None,
        }
    }

    pub fn take_time_delta(&mut self) -> Option<(usize, i32)> {
        self.time_delta.take()
    }

    pub fn led_strip(&self) -> &LedStripState {
        &self.led_strip
    }

    pub fn last_mode(&self) -> AppMode {
        self.last_mode
    }

    pub fn mode(&self) -> AppMode {
        self.mode
    }

    pub fn brightness(&self) -> u32 {
        self.brightness
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
        self.last_mode = self.mode;

        match mode {
            Some(ButtonEvent::Release) => self.is_mode_down = false,
            Some(ButtonEvent::Press) => {
                self.is_mode_down = true;
                self.lr_pressed_while_mode_down = false;
            }
            _ => {}
        }

        let mode = matches!(mode, Some(ButtonEvent::Release));
        let left = matches!(left, Some(ButtonEvent::Release));
        let right = matches!(right, Some(ButtonEvent::Release));
        match self.mode {
            AppMode::Regular(ref mut screen) => {
                if mode {
                    self.transition(AppMode::Menu(MenuOption::Return));
                } else if left {
                    *screen = screen.left();
                    self.transition = true;
                } else if right {
                    *screen = screen.right();
                    self.transition = true;
                }
            }
            AppMode::Menu(menu) => {
                if mode {
                    self.transition(match menu {
                        MenuOption::Return => AppMode::Regular(Default::default()),
                        MenuOption::SetTime => AppMode::SetTime(Default::default()),
                        MenuOption::SetAlarm => AppMode::SetAlarm(Default::default()),
                        MenuOption::SetRgb => AppMode::SetRgb,
                        MenuOption::SetBrightness => AppMode::SetBrightness,
                        MenuOption::TempHumidity => AppMode::TempHumidity,
                    });
                } else if left {
                    self.transition(AppMode::Menu(menu.left()));
                } else if right {
                    self.transition(AppMode::Menu(menu.right()));
                }
            }
            AppMode::SetTime(ref mut screen_index) => {
                if self.is_mode_down {
                    if left {
                        self.time_delta = Some((*screen_index, -1));
                        self.lr_pressed_while_mode_down = true;
                    } else if right {
                        self.time_delta = Some((*screen_index, 1));
                        self.lr_pressed_while_mode_down = true;
                    }
                } else if left {
                    if *screen_index == 0 {
                        *screen_index = 11;
                    } else {
                        *screen_index -= 1;
                    }
                    self.transition = true;
                } else if right {
                    if *screen_index == 11 {
                        *screen_index = 0;
                    } else {
                        *screen_index += 1;
                    }
                    self.transition = true;
                }

                if mode && !self.lr_pressed_while_mode_down {
                    self.transition_regular();
                }
            }
            AppMode::SetAlarm(ref mut screen_index) => {
                if self.is_mode_down {
                    if left {
                        self.time_delta = Some((*screen_index, -1));
                        self.lr_pressed_while_mode_down = true;
                    } else if right {
                        self.time_delta = Some((*screen_index, 1));
                        self.lr_pressed_while_mode_down = true;
                    }
                } else if left {
                    if *screen_index == 0 {
                        *screen_index = 11;
                    } else {
                        *screen_index -= 1;
                    }
                    self.transition = true;
                } else if right {
                    if *screen_index == 11 {
                        *screen_index = 0;
                    } else {
                        *screen_index += 1;
                    }
                    self.transition = true;
                }

                if mode && !self.lr_pressed_while_mode_down {
                    self.transition_regular();
                }
            }
            AppMode::SetRgb => {
                if left {
                    self.led_strip.left();
                    self.transition = true;
                } else if right {
                    self.led_strip.right();
                    self.transition = true;
                }

                if mode {
                    self.transition_regular();
                }
            }
            AppMode::SetBrightness => {
                if left {
                    self.brightness = self.brightness.saturating_sub(1);
                    self.transition = true;
                } else if right {
                    self.brightness = core::cmp::min(9, self.brightness + 1);
                    self.transition = true;
                }

                if mode {
                    self.transition_regular();
                }
            }
            AppMode::TempHumidity => {
                todo!()
            }
        }
    }

    pub fn update(&mut self) {
        self.led_strip.update();
    }

    fn transition(&mut self, mode: AppMode) {
        self.mode = mode;
        self.transition = true;
    }

    fn transition_regular(&mut self) {
        self.transition(AppMode::Regular(Default::default()));
    }
}
