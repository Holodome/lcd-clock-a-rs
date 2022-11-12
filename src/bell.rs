use crate::hal::pwm::Slice;
use embedded_hal::PwmPin;
use rp_pico::hal::pwm::{SliceId, SliceMode, ValidSliceMode};

/// Frequency of Low C notes
const CL: [u16; 8] = [0, 131, 147, 165, 175, 196, 211, 248];
/// Frequency of Middle C notes
const CM: [u16; 8] = [0, 262, 294, 330, 349, 392, 440, 494];
/// Frequency of High C notes
const CH: [u16; 8] = [0, 525, 589, 661, 700, 786, 882, 990];

struct Song<const N: usize> {
    notes: [u16; N],
    beats: [u8; N],
}

const SONG1: Song<31> = Song {
    notes: [
        CM[3], CM[5], CM[6], CM[3], CM[2], CM[3], CM[5], CM[6], CH[1], CM[6], CM[5], CM[1], CM[3],
        CM[2], CM[2], CM[3], CM[5], CM[2], CM[3], CM[3], CL[6], CL[6], CL[6], CM[1], CM[2], CM[3],
        CM[2], CL[7], CL[6], CM[1], CL[5],
    ],
    beats: [
        1, 1, 3, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1,
    ],
};

const SONG2: Song<30> = Song {
    notes: [
        CM[1], CM[1], CM[1], CL[5], CM[3], CM[3], CM[3], CM[1], CM[1], CM[3], CM[5], CM[5], CM[4],
        CM[3], CM[2], CM[2], CM[3], CM[4], CM[4], CM[3], CM[2], CM[3], CM[1], CM[1], CM[3], CM[2],
        CL[5], CL[7], CM[2], CM[1],
    ],
    beats: [
        1, 1, 2, 2, 1, 1, 2, 2, 1, 1, 2, 2, 1, 1, 3, 1, 1, 2, 2, 1, 1, 2, 2, 1, 1, 2, 2, 1, 1, 3,
    ],
};

const SONG3: Song<49> = Song {
    notes: [
        CM[1], CM[2], CM[3], CM[5], CM[5], CM[0], CM[3], CM[2], CM[1], CM[2], CM[3], CM[0], CM[1],
        CM[2], CM[3], CM[7], CH[1], CH[1], CH[1], CM[7], CH[1], CM[7], CM[6], CM[5], CM[0], CM[1],
        CM[2], CM[3], CM[5], CM[5], CM[0], CM[3], CM[2], CM[1], CM[2], CM[1], CM[0], CM[1], CM[2],
        CM[3], CM[5], CM[1], CM[0], CM[1], CL[7], CL[6], CL[7], CM[1], CM[0],
    ],
    beats: [
        2, 2, 2, 1, 5, 4, 2, 2, 2, 1, 5, 4, 2, 2, 2, 1, 5, 2, 2, 2, 1, 3, 2, 4, 4, 2, 2, 2, 1, 5,
        4, 2, 2, 2, 1, 3, 5, 2, 2, 2, 1, 5, 4, 2, 2, 2, 2, 8, 2,
    ],
};

pub struct Bell<PWM, PIN> {
    pwm: PWM,
    pin: PIN,
}

impl<PWM, PIN> Bell<PWM, PIN>
where
    PWM: PwmPin<Duty = u16>,
{
    pub fn beep(&mut self, freq: u32) {
        // let max = set_pwm_period(&mut self.slice, self.sysclk, freq);
        // self.pwm.set_duty(max);
        // self.pwm.set_duty(0);
    }
}

pub fn set_pwm_period<I: SliceId, M: SliceMode + ValidSliceMode<I>>(
    slice: &mut Slice<I, M>,
    sysclk: u32,
    freq: u32,
) -> u16 {
    // div_frac = 0
    // ph_correct = 0
    // div_int = 125
    // freq = [0, 1000]
    // period = [0, 125_000]
    // period = (top + 1) * 125
    // (top + 1) = [0, 1000]
    // top = period / 125 - 1
    let period = sysclk / freq;
    let div_int = 125;
    let top = (period / div_int) as u16;
    slice.set_top(top);
    slice.clr_ph_correct();
    slice.set_div_int(125);
    slice.set_div_frac(0);

    top
}
