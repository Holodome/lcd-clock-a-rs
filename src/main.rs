#![no_std]
#![no_main]

// use embedded_hal::{Pwm, PwmPin};
use panic_halt as _;

use cortex_m::delay::Delay;
use embedded_hal::spi::MODE_0;
use fugit::*;
use rp_pico::{
    entry,
    hal::{
        self,
        clocks::{init_clocks_and_plls, Clock},
        gpio,
        pac::{CorePeripherals, Peripherals},
        spi::Spi,
        watchdog::Watchdog,
        Sio,
    },
    Pins,
};

mod bell;
mod pins;
mod st7789vwx6;

use st7789vwx6::{Display, ST7789VWx6};

#[entry]
fn main() -> ! {
    let mut dp = Peripherals::take().unwrap();
    let cp = CorePeripherals::take().unwrap();

    let mut wdg = Watchdog::new(dp.WATCHDOG);
    let sio = Sio::new(dp.SIO);

    let clocks = init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
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

    let mut delay = Delay::new(cp.SYST, clocks.system_clock.freq().to_Hz());

    let csa1 = pins.gpio2.into_push_pull_output();
    let csa2 = pins.gpio3.into_push_pull_output();
    let csa3 = pins.gpio4.into_push_pull_output();
    let dc = pins.gpio8.into_push_pull_output();
    let rst = pins.gpio12.into_push_pull_output();
    let _clk = pins.gpio9.into_mode::<gpio::FunctionSpi>();
    let _miso = pins.gpio10.into_mode::<gpio::FunctionSpi>();
    let _mosi = pins.gpio11.into_mode::<gpio::FunctionSpi>();

    let spi = Spi::<_, _, 8>::new(dp.SPI1);
    let spi = spi.init(
        &mut dp.RESETS,
        clocks.peripheral_clock.freq(),
        40_000_000u32.Hz(),
        &MODE_0,
    );

    let mut st7789 = ST7789VWx6::new((csa1, csa2, csa3, dc, rst), spi);
    st7789.init(&mut delay).unwrap();

    for x in 0..135 {
        for y in 0..240 {
            st7789.set_pixel(Display::D1, x, y, 0x00).unwrap();
        }
    }

    // let buzzer = pins.gpio14.into_push_pull_output();

    // let mut pwm_slices = hal::pwm::Slices::new(dp.PWM, &mut dp.RESETS);
    // let pwm = &mut pwm_slices.pwm7;
    // pwm.set_ph_correct();
    // pwm.enable();

    // let channel = &mut pwm.channel_a;
    // channel.output_to(buzzer);

    // channel.set_duty(u16::MAX);
    // delay.delay_ms(1000);
    // channel.set_duty(0);

    loop {}
}
