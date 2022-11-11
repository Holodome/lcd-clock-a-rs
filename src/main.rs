#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m::delay::Delay;
use embedded_hal::{digital::v2::OutputPin, spi::MODE_0};
use fugit::*;
use hal::{
    entry,
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio,
        pac::{CorePeripherals, Peripherals},
        spi::Spi,
        watchdog::Watchdog,
        Sio,
    },
    Pins,
};
use rp_pico as hal;

mod st7789vwx6;

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

    // self.DC_PIN = 25
    //    self.BL_PIN = 24
    //    self.RST_PIN = 27

    //    self.CSA1_PIN = 16  # 74HC138 a1
    //    self.CSA2_PIN = 20
    //    self.CSA3_PIN = 21

    let mut led = pins.led.into_push_pull_output();
    let mut delay = Delay::new(cp.SYST, clocks.system_clock.freq().to_Hz());

    let _spi_sclk = pins.gpio2.into_mode::<gpio::FunctionSpi>();
    let _spi_mosi = pins.gpio3.into_mode::<gpio::FunctionSpi>();
    let _spi_miso = pins.gpio4.into_mode::<gpio::FunctionSpi>();
    let spi = Spi::<_, _, 8>::new(dp.SPI0);
    let spi = spi.init(
        &mut dp.RESETS,
        clocks.peripheral_clock.freq(),
        40_000_000u32.Hz(),
        &MODE_0,
    );

    loop {
        led.set_high().ok();
        delay.delay_ms(500);
        led.set_low().ok();
        delay.delay_ms(500);
    }
}
