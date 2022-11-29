#![no_std]
#![no_main]

use lcd_clock::LcdClock;
#[cfg(not(feature = "semihosting"))]
use panic_halt as _;
#[cfg(feature = "semihosting")]
use panic_semihosting as _;

#[cfg(feature = "semihosting")]
#[macro_use]
extern crate cortex_m_semihosting;

use embedded_hal::{spi::MODE_0, timer::CountDown};
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
mod bme280;
mod ds3231;
mod images;
mod lcd_clock;
mod st7789vwx6;
mod ws2812;

use crate::{
    bme280::{BME280State, BME280},
    ds3231::{DS3231State, Time, DS3231},
    lcd_clock::{BME280_I2C_ADDR, DS3231_I2C_ADDR},
    st7789vwx6::{Display, ST7789VWx6},
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
    let mut pwm_slices = hal::pwm::Slices::new(dp.PWM, &mut dp.RESETS);

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

    let ds3231_state = DS3231State::new(DS3231_I2C_ADDR);
    let bme280_state = BME280State::new(BME280_I2C_ADDR);

    let mut st7789vw = {
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

    let mut ws2812 = {
        let (mut pio, sm0, _, _, _) = dp.PIO0.split(&mut dp.RESETS);
        let mut rgb = pins.gpio22.into_mode();
        WS2812::new(6, rgb, &mut pio, sm0, clocks.peripheral_clock.freq()).unwrap()
    };

    st7789vw.init().unwrap();
    st7789vw.clear_all(0).unwrap();

    let (i2c_bus, bme280_state) = {
        let mut bme280 = BME280::new(i2c_bus, bme280_state);
        bme280.init().unwrap();
        bme280.release()
    };
    let (i2c_bus, ds3231_state) = {
        let mut ds3231 = DS3231::new(i2c_bus, ds3231_state);
        ds3231.init().unwrap();
        ds3231.release()
    };

    let lcd_clock = LcdClock::new(i2c_bus, ds3231_state, bme280_state, st7789vw);

    loop {
        ws2812.display(255, 0, 255);
        // if time != prev_time {
        //     st7789
        //         .set_pixels(
        //             Display::D1,
        //             0,
        //             0,
        //             st7789.width(),
        //             st7789.height(),
        //             images::NUMPIC_A.get_digit(time.hours /
        // 10).unwrap().data(),         )
        //         .unwrap();
        //     st7789
        //         .set_pixels(
        //             Display::D2,
        //             0,
        //             0,
        //             st7789.width(),
        //             st7789.height(),
        //             images::NUMPIC_A.get_digit(time.hours %
        // 10).unwrap().data(),         )
        //         .unwrap();
        //     st7789
        //         .set_pixels(
        //             Display::D3,
        //             0,
        //             0,
        //             st7789.width(),
        //             st7789.height(),
        //             images::NUMPIC_A.get_digit(time.mins /
        // 10).unwrap().data(),         )
        //         .unwrap();
        //     st7789
        //         .set_pixels(
        //             Display::D4,
        //             0,
        //             0,
        //             st7789.width(),
        //             st7789.height(),
        //             images::NUMPIC_A.get_digit(time.mins %
        // 10).unwrap().data(),         )
        //         .unwrap();
        //     st7789
        //         .set_pixels(
        //             Display::D5,
        //             0,
        //             0,
        //             st7789.width(),
        //             st7789.height(),
        //             images::NUMPIC_A.get_digit(time.secs /
        // 10).unwrap().data(),         )
        //         .unwrap();
        //     st7789
        //         .set_pixels(
        //             Display::D6,
        //             0,
        //             0,
        //             st7789.width(),
        //             st7789.height(),
        //             images::NUMPIC_A.get_digit(time.secs %
        // 10).unwrap().data(),         )
        //         .unwrap();
        // }

        // hprintln!("here");
        //
    }
}
