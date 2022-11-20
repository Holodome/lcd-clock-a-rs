#![no_std]
#![no_main]

#[cfg(not(feature = "semihosting"))]
use panic_halt as _;
#[cfg(feature = "semihosting")]
use panic_semihosting as _;

#[cfg(feature = "semihosting")]
#[macro_use]
extern crate cortex_m_semihosting;

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
        pio::PIOExt,
        spi::Spi,
        watchdog::Watchdog,
        Sio,
    },
    Pins,
};

mod bell;
mod ds3231;
mod images;
mod lcd_clock;
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

    let mut ds3231 = {
        let sda = pins.gpio6.into_mode::<gpio::FunctionI2C>();
        let scl = pins.gpio7.into_mode::<gpio::FunctionI2C>();
        let i2c = hal::I2C::i2c1(
            dp.I2C1,
            sda,
            scl,
            100u32.kHz(),
            &mut dp.RESETS,
            &clocks.peripheral_clock,
        );
        ds3231::DS3231::new(i2c, ds3231::ADDRESS)
    };
    ds3231.init().unwrap();
    hprintln!("{:?}", ds3231.get_calendar().unwrap());
    hprintln!("{:?}", ds3231.get_time().unwrap());

    // let csa2 = pins.gpio3.into_push_pull_output();
    // let csa3 = pins.gpio4.into_push_pull_output();
    // let dc = pins.gpio8.into_push_pull_output();
    // let rst = pins.gpio12.into_push_pull_output();
    // let _clk = pins.gpio9.into_mode::<gpio::FunctionSpi>();
    // let _miso = pins.gpio10.into_mode::<gpio::FunctionSpi>();
    // let _mosi = pins.gpio11.into_mode::<gpio::FunctionSpi>();

    // let spi = Spi::<_, _, 8>::new(dp.SPI1);
    // let spi = spi.init(
    //     &mut dp.RESETS,
    //     clocks.peripheral_clock.freq(),
    //     40_000_000u32.Hz(),
    //     &MODE_0,
    // );

    // let mut st7789 = ST7789VWx6::new((csa1, csa2, csa3, dc, rst), spi);
    // st7789.init(&mut delay).unwrap();
    // st7789.clear_all(0xFFFF).unwrap();

    //     for x in 0..135 {
    //         for y in 0..240 {
    //             for display in [Display::D1] {
    //                 st7789.set_pixel(display, x, y, 0xFF00).unwrap();
    //             }
    //         }
    //     }

    loop {}
}
