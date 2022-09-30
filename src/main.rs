use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::ledc::*;
use esp_idf_hal::peripherals::Peripherals;
use std::sync::Arc;
use std::sync::Mutex;

type ChannelA<'a> = Arc<Mutex<LedcDriver<'a, CHANNEL0, LedcTimerDriver<'a, TIMER0>>>>;
type ChannelB<'a> = Arc<Mutex<LedcDriver<'a, CHANNEL1, LedcTimerDriver<'a, TIMER1>>>>;

const PWM_HOLD_RATE_LOW: u32 = 2;
// const PWM_HOLD_RATE_HIGH: u32 = 10;
const CHANGEOVER_MS: u32 = 200;

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    println!("Configuring output channel GPIO 1");

    let peripherals = Peripherals::take().unwrap();
    let config = config::TimerConfig::new().frequency(1000.into());
    let channel_a: ChannelA = Arc::new(Mutex::new(LedcDriver::new(
        peripherals.ledc.channel0,
        LedcTimerDriver::new(peripherals.ledc.timer0, &config)?,
        peripherals.pins.gpio1,
        &config,
    )?));
    let channel_b: ChannelB = Arc::new(Mutex::new(LedcDriver::new(
        peripherals.ledc.channel1,
        LedcTimerDriver::new(peripherals.ledc.timer1, &config)?,
        peripherals.pins.gpio2,
        &config,
    )?));

    println!("Configuring input channel A GPIO 0 (active low)");
    let mut sense_a = PinDriver::input(peripherals.pins.gpio0)?;
    sense_a.set_pull(Pull::Down)?;

    // println!("Configuring input channel B GPIO 1 (active low)");
    // let mut sense_b = PinDriver::input(peripherals.pins.gpio1)?;
    // sense_b.set_pull(Pull::Up)?;

    println!("Configuring output channel LED GPIO 7 (active high)");
    let led = Arc::new(Mutex::new(PinDriver::output(peripherals.pins.gpio7)?));
    // led.set_low()?;

    println!("Configuring input PWM sense (active high)");
    let mut pwm_sense = PinDriver::input(peripherals.pins.gpio5)?;
    pwm_sense.set_pull(Pull::Down)?;

    // println!("Configuring input PWM sense (active low)");
    // let mut pwm_signal = PinDriver::output(peripherals.pins.gpio5)?;
    // pwm_signal.set_high()?;

    println!("Starting contactor control loop");
    let mut state_a = false;
    // let mut state_b = false;
    let mut pwm_hold_rate = 60;

    // activate_contactor_a(channel_a.clone(), 255)?;
    // FreeRtos::delay_ms(200);
    // activate_contactor_a(channel_a.clone(), pwm_hold_rate)?;
    loop {
        if pwm_sense.is_high() {
            pwm_hold_rate += 1
        }
        led.clone().lock().unwrap().toggle().unwrap();
        FreeRtos::delay_ms(500);
        if sense_a.is_low() && !state_a {
            println!("State A: Debounce 100ms started");
            FreeRtos::delay_ms(200);
            if sense_a.is_low() {
                activate_contactor_a(channel_a.clone(), pwm_hold_rate)?;
                activate_contactor_b(channel_b.clone(), pwm_hold_rate)?;
                state_a = true;
            }

            // continue;
        } else if sense_a.is_low() && state_a {
            // activate_contactor_a(channel_a.clone(), pwm_hold_rate)?;
            println!(
                "State A: {state_a} Output:{:?}/256",
                channel_a.clone().lock().unwrap().get_duty()
            );
            println!(
                "State B: {state_a} Output:{:?}/256",
                channel_b.clone().lock().unwrap().get_duty()
            );
        } else {
            deactivate_contactor_a(channel_a.clone())?;
            state_a = false;
            deactivate_contactor_b(channel_b.clone())?;
            state_a = false;
        }

        // led_thread.join().unwrap()?;
    }
}

fn deactivate_contactor_a(channel_a: ChannelA) -> anyhow::Result<()> {
    if let Ok(mut channel) = channel_a.lock() {
        if channel.disable().is_ok() {
            println!("Contactor A disabled");
        } else {
            println!("Contactor A disable fault");
        }
    }
    Ok(())
}
fn deactivate_contactor_b(channel_b: ChannelB) -> anyhow::Result<()> {
    if let Ok(mut channel) = channel_b.lock() {
        if channel.disable().is_ok() {
            println!("Contactor B disabled");
        } else {
            println!("Contactor B disable fault");
        }
    }
    Ok(())
}

fn activate_contactor_a(channel_a: ChannelA, pwm: u32) -> Result<(), anyhow::Error> {
    if let Ok(mut channel) = channel_a.lock() {
        let max_duty = channel.get_max_duty();
        channel.set_duty(max_duty)?;
        channel.enable()?;
        println!("Conactor A: Set duty 100%");
        FreeRtos::delay_ms(CHANGEOVER_MS);
        return Ok(channel.set_duty(pwm)?);
    };
    Err(anyhow::anyhow!("Conactor A: Failed to get mutex"))
}

fn activate_contactor_b(channel_b: ChannelB, pwm: u32) -> Result<(), anyhow::Error> {
    if let Ok(mut channel) = channel_b.lock() {
        let max_duty = channel.get_max_duty();
        channel.set_duty(max_duty)?;
        channel.enable()?;
        println!("Conactor B: Set duty 100%");
        FreeRtos::delay_ms(CHANGEOVER_MS);
        return Ok(channel.set_duty(pwm)?);
    };
    Err(anyhow::anyhow!("Conactor B: Failed to get mutex"))
}

fn update_led(
    s1: bool,
    s2: bool,
    led_a: Arc<Mutex<PinDriver<Gpio7, Output>>>,
) -> anyhow::Result<()> {
    if let Ok(mut led) = led_a.lock() {
        if s1 {
            led.set_high()?;
            FreeRtos::delay_ms(100);
            led.toggle()?;
            FreeRtos::delay_ms(100);
        } else {
            led.set_low()?;
            FreeRtos::delay_ms(200);
        }

        FreeRtos::delay_ms(600);
        if s2 {
            led.set_high()?;
            FreeRtos::delay_ms(50);
            led.toggle()?;
            FreeRtos::delay_ms(50);

            led.set_high()?;
            FreeRtos::delay_ms(50);
            led.toggle()?;
            FreeRtos::delay_ms(50);
        } else {
            led.set_low()?;
            FreeRtos::delay_ms(200);
        }
        if !s1 && !s2 {
            for _ in 0..12 {
                led.set_high()?;
                FreeRtos::delay_ms(50);
                led.set_low()?;
                FreeRtos::delay_ms(50);
            }
        } else {
            FreeRtos::delay_ms(600);
        }
    }
    Ok(())
}
