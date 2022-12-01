//! WS2812 PIO

use crate::hal::{
    self,
    gpio::{Function, FunctionConfig, Pin, PinId, ValidPinMode},
    pio::{PIOExt, StateMachineIndex, Tx, UninitStateMachine, PIO},
};
use fugit::HertzU32;

pub struct WS2812<P, SM, I>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    SM: StateMachineIndex,
{
    tx: Tx<(P, SM)>,
    _pin: Pin<I, Function<P>>,

    led_count: usize,
}

impl<P, SM, I> WS2812<P, SM, I>
where
    I: PinId,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    SM: StateMachineIndex,
{
    pub fn new(
        led_count: usize,
        pin: Pin<I, Function<P>>,
        pio: &mut PIO<P>,
        sm: UninitStateMachine<(P, SM)>,
        clock_freq: fugit::HertzU32,
    ) -> Result<Self, Error> {
        const T1: u8 = 2; // start bit
        const T2: u8 = 5; // data bit
        const T3: u8 = 3; // stop bit
        const CYCLES_PER_BIT: u32 = (T1 + T2 + T3) as u32;
        const FREQ: HertzU32 = HertzU32::kHz(800);

        let program = {
            let side_set = pio::SideSet::new(false, 1, false);
            let mut a = pio::Assembler::new_with_side_set(side_set);

            let mut wrap_target = a.label();
            let mut wrap_source = a.label();
            let mut do_zero = a.label();

            // This PIO program shifts all bits of source repeatedly until it is
            // zero while maintaining timings
            /*
             * wrap_target:
             *  out x, 1        [T3 - 1]
             *  jmp !x do_zero  [T1 - 1]
             *  jmp wrap_target [T2 - 1]
             * do_zero:
             *  nop             [T2 - 1]
             * wrap_source:
             */

            a.bind(&mut wrap_target);
            a.out_with_delay_and_side_set(pio::OutDestination::X, 1, T3 - 1, 0);
            a.jmp_with_delay_and_side_set(pio::JmpCondition::XIsZero, &mut do_zero, T1 - 1, 1);
            a.jmp_with_delay_and_side_set(pio::JmpCondition::Always, &mut wrap_target, T2 - 1, 1);
            a.bind(&mut do_zero);
            a.nop_with_delay_and_side_set(T2 - 1, 0);
            a.bind(&mut wrap_source);

            let program = a.assemble_with_wrap(wrap_source, wrap_target);
            pio.install(&program).map_err(|_| Error::PioError)?
        };

        let bit_freq = FREQ * CYCLES_PER_BIT;
        let int = clock_freq / bit_freq;
        let rem = clock_freq - (int * bit_freq);
        let frac = (rem * 256) / bit_freq;

        let int: u16 = int as u16;
        let frac: u8 = frac as u8;

        let (mut sm, _, tx) = hal::pio::PIOBuilder::from_program(program)
            .buffers(hal::pio::Buffers::OnlyTx)
            .side_set_pin_base(I::DYN.num)
            .out_shift_direction(hal::pio::ShiftDirection::Left)
            .autopull(true)
            .pull_threshold(24)
            // .clock_divisor_fixed_point(int, frac) NOTE: rp-2040 0.7
            .clock_divisor(int as f32 + (frac as f32) / 256.0)
            .build(sm);

        sm.set_pindirs([(I::DYN.num, hal::pio::PinDir::Output)]);
        sm.start();

        Ok(Self {
            led_count,
            tx,
            _pin: pin,
        })
    }

    pub fn display(&mut self, r: u8, g: u8, b: u8) {
        let word = (u32::from(g) << 24) | (u32::from(r) << 16) | (u32::from(b) << 8);
        for _ in 0..self.led_count {
            while !self.tx.write(word) {
                cortex_m::asm::nop();
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    PioError,
}
