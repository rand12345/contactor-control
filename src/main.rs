// use embedded_hal::delay::blocking::DelayUs;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::ledc::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();

    println!("Configuring output channel");

    let peripherals = Peripherals::take().unwrap();
    let config = config::TimerConfig::new().frequency(2.kHz().into());
    let mut channel = LedcDriver::new(
        peripherals.ledc.channel0,
        LedcTimerDriver::new(peripherals.ledc.timer0, &config)?,
        peripherals.pins.gpio4,
        &config,
    )?;

    println!("Starting duty-cycle loop");

    let max_duty = channel.get_max_duty();
    for numerator in (1..=10).into_iter().cycle() {
        println!("Duty {}/10", numerator);
        channel.set_duty(max_duty * numerator / 10)?;
        FreeRtos::delay_ms(2000);
    }

    loop {
        FreeRtos::delay_ms(1000);
    }
}
