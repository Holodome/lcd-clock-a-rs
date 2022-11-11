#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m::delay::Delay;
use embedded_hal::digital::v2::OutputPin;
use hal::hal::Sio;
use hal::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac::{CorePeripherals, Peripherals},
    watchdog::Watchdog,
};
use hal::{entry, Pins};
use rp_pico as hal;

#[entry]
fn main() -> ! {
    let mut dp = Peripherals::take().unwrap();
    let cp = CorePeripherals::take().unwrap();

    let mut wdg = Watchdog::new(dp.WATCHDOG);
    let sio = Sio::new(dp.SIO);

    let clocks = init_clocks_and_plls(
        12_000_000,
        dp.XOSC,
        dp.CLOCKS,
        dp.PLL_SYS,
        dp.PLL_USB,
        &mut dp.RESETS,
        &mut wdg,
    )
    .ok()
    .unwrap();

    let pins = Pins::new(dp.IO_BANK0, dp.PADS_BANK0, sio.gpio_bank0, &mut dp.RESETS);

    let mut led = pins.led.into_push_pull_output();
    let mut delay = Delay::new(cp.SYST, clocks.system_clock.freq().to_Hz());

    loop {
        led.set_high().ok();
        delay.delay_ms(500);
        led.set_low().ok();
        delay.delay_ms(500);
    }
}
