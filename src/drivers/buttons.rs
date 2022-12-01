use core::convert::Infallible;
use embedded_hal::digital::v2::InputPin;
use unwrap_infallible::UnwrapInfallible;

#[derive(Debug, Clone, Copy)]
pub enum ButtonEvent {
    Press,
    Release,
}

#[derive(Debug, Clone, Copy)]
pub enum ButtonState {
    Released,
    Pressed,
}

pub struct Button<P>
where
    P: InputPin,
{
    pin: Debounce<P>,
    state: ButtonState,
}

impl<P> Button<P>
where
    P: InputPin<Error = Infallible>,
{
    pub fn new(pin: Debounce<P>) -> Self {
        Self {
            pin,
            state: ButtonState::Released,
        }
    }

    pub fn is_pressed(&self) -> bool {
        self.pin.is_pressed()
    }

    pub fn update(&mut self) -> Option<ButtonEvent> {
        self.pin.update();
        match self.state {
            ButtonState::Released => {
                if self.pin.is_pressed() {
                    self.state = ButtonState::Pressed;
                    return Some(ButtonEvent::Press);
                }
            }
            ButtonState::Pressed => {
                if !self.pin.is_pressed() {
                    self.state = ButtonState::Released;
                    return Some(ButtonEvent::Release);
                }
            }
        }

        None
    }
}

pub struct Debounce<P>
where
    P: InputPin,
{
    pin: P,
    integrator: u32,
    max: u32,
    output: bool,
}

impl<P> Debounce<P>
where
    P: InputPin<Error = Infallible>,
{
    pub fn new(pin: P, integrator_max: u32) -> Self {
        Self {
            pin,
            integrator: 0,
            max: integrator_max,
            output: false,
        }
    }

    pub fn is_pressed(&self) -> bool {
        self.output
    }

    pub fn update(&mut self) {
        if self.pin.is_low().unwrap_infallible() && self.integrator != 0 {
            self.integrator -= 1;
        } else if self.integrator < self.max {
            self.integrator += 1;
        }

        if self.integrator == 0 {
            self.output = false;
        } else if self.integrator == self.max {
            self.output = true;
        }
    }
}
