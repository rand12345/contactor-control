use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::ledc::*;
use esp_idf_hal::peripherals::Peripherals;
use std::sync::Arc;
use std::sync::Mutex;

type Channel<'a> = Arc<Mutex<LedcDriver<'a, CHANNEL0, LedcTimerDriver<'a, TIMER0>>>>;
fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    println!("Configuring output channel GPIO 1");

    let peripherals = Peripherals::take().unwrap();
    let config = config::TimerConfig::new().frequency(1000.into());
    let channel: Channel = Arc::new(Mutex::new(LedcDriver::new(
        peripherals.ledc.channel0,
        LedcTimerDriver::new(peripherals.ledc.timer0, &config)?,
        peripherals.pins.gpio1,
        &config,
    )?));
    println!("Configuring input channel GPIO 0 (active low)");
    let mut sense = PinDriver::input(peripherals.pins.gpio0)?;
    sense.set_pull(Pull::Up)?;

    println!("Starting contactor control loop");
    let mut state = false;
    loop {
        if sense.is_low() && !state {
            println!("Debounce 100ms started");
            FreeRtos::delay_ms(100);
            if sense.is_low() {
                activate_contactor(channel.clone())?;
                state = true;
            }

            continue;
        } else if sense.is_low() && state {
            println!(
                "State: {state} Output:{:?}/256",
                channel.clone().lock().unwrap().get_duty()
            );
        } else {
            deactivate_contactor(channel.clone())?;
            state = false;
        }
        FreeRtos::delay_ms(1000);
    }
}

fn deactivate_contactor(channel_a: Channel) -> anyhow::Result<()> {
    if let Ok(mut channel) = channel_a.lock() {
        if channel.disable().is_ok() {
            println!("Contactor disabled");
        } else {
            println!("Contactor disable fault");
        }
    }
    Ok(())
}

fn activate_contactor(channel_a: Channel) -> Result<(), anyhow::Error> {
    if let Ok(mut channel) = channel_a.lock() {
        let max_duty = channel.get_max_duty();
        channel.set_duty(max_duty)?;
        channel.enable()?;
        println!("Set duty 100%");
        FreeRtos::delay_ms(200);
        return Ok(channel.set_duty(1)?);
    };
    Err(anyhow::anyhow!("Failed to get mutex"))
}
