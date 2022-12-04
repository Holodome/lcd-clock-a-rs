#![no_std]
#![no_main]

use drivers::buttons::{Button, Debounce};
use lcd_clock::{LcdClock, LcdClockHardware};
#[cfg(not(feature = "semihosting"))]
use panic_halt as _;
#[cfg(feature = "semihosting")]
use panic_semihosting as _;

#[cfg(feature = "semihosting")]
#[macro_use]
extern crate cortex_m_semihosting;

use embedded_hal::spi::MODE_0;
use fugit::*;
use rp_pico::{
    entry,
    hal::{
        self,
        clocks::{init_clocks_and_plls, Clock},
        gpio,
        pac::Peripherals,
        pio::PIOExt,
        spi::Spi,
        watchdog::Watchdog,
        Sio,
    },
    Pins,
};

mod bell;
mod drivers;
mod images;
mod lcd_clock;
mod led_strip;
mod misc;

use crate::drivers::{
    st7789vwx6::{self, ST7789VWx6},
    ws2812::WS2812,
};

#[entry]
fn main() -> ! {
    let mut dp = Peripherals::take().unwrap();

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
    let pwm_slices = hal::pwm::Slices::new(dp.PWM, &mut dp.RESETS);

    let i2c_bus = {
        let sda = pins.gpio6.into_mode::<gpio::FunctionI2C>();
        let scl = pins.gpio7.into_mode::<gpio::FunctionI2C>();
        hal::I2C::i2c1(
            dp.I2C1,
            sda,
            scl,
            100u32.kHz(),
            &mut dp.RESETS,
            &clocks.peripheral_clock,
        )
    };

    let st7789vw = {
        let csa1 = pins.gpio2.into_push_pull_output();
        let csa2 = pins.gpio3.into_push_pull_output();
        let csa3 = pins.gpio4.into_push_pull_output();
        let dc = pins.gpio8.into_push_pull_output();
        let rst = pins.gpio12.into_push_pull_output();
        let _clk = pins.gpio9.into_mode::<gpio::FunctionSpi>();
        let _miso = pins.gpio10.into_mode::<gpio::FunctionSpi>();
        let _mosi = pins.gpio11.into_mode::<gpio::FunctionSpi>();
        let bl = pins.gpio13.into_push_pull_output();

        let mut pwm = pwm_slices.pwm6;
        pwm.set_ph_correct();
        pwm.enable();

        let mut channel = pwm.channel_b;
        channel.output_to(bl);

        let spi = Spi::<_, _, 8>::new(dp.SPI1);
        let spi = spi.init(
            &mut dp.RESETS,
            clocks.peripheral_clock.freq(),
            40_000_000u32.Hz(),
            &MODE_0,
        );

        ST7789VWx6::new(
            (csa1, csa2, csa3, dc, rst),
            spi,
            channel,
            st7789vwx6::WIDTH,
            st7789vwx6::HEIGHT,
            u16::MAX / 5,
        )
    };

    let ws2812 = {
        let (mut pio, sm0, _, _, _) = dp.PIO0.split(&mut dp.RESETS);
        let rgb = pins.gpio22.into_mode();
        WS2812::new(rgb, &mut pio, sm0, clocks.peripheral_clock.freq()).unwrap()
    };

    let button_debounce_integrator = 2;
    let button_right = Button::new(Debounce::new(
        pins.gpio15.into_pull_down_input(),
        button_debounce_integrator,
    ));
    let button_left = Button::new(Debounce::new(
        pins.gpio16.into_pull_down_input(),
        button_debounce_integrator,
    ));
    let button_mode = Button::new(Debounce::new(
        pins.gpio17.into_pull_down_input(),
        button_debounce_integrator,
    ));

    let hardware = LcdClockHardware::new(
        i2c_bus,
        st7789vw,
        ws2812,
        button_right,
        button_left,
        button_mode,
        (),
    );

    let sin = hal::rom_data::float_funcs::fsin::ptr();
    let mut lcd_clock = LcdClock::new(hardware, sin);

    // delay for 2ms so displays are initialized
    cortex_m::asm::delay(125 * 1000 * 20);
    lcd_clock.init().unwrap();

    loop {
        lcd_clock.update().unwrap();
    }
}
