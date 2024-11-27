pub mod osc;
pub mod wifi;
use osc::Osc;
use wifi::*;

use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use log::*;

const WIFI_SSID: &str = env!("OSC_WIFI_SSID");
const WIFI_PASS: &str = env!("OSC_WIFI_PASS");
const OSC_PORT: &str = env!("OSC_WIFI_RECV_PORT");

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Setup Wifi

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // Create thread to get heart rate
    // put tx in this heart rate thread
    // (don't put it in the function, that will poll the heart rate sensor, but the thread)
    // use std::sync::mpsc;
    // use std::thread;

    // let (tx, rx) = mpsc::channel();

    let mut led = PinDriver::output(peripherals.pins.gpio20)?;
    let mut heartbeat = PinDriver::input(peripherals.pins.gpio3)?;

    heartbeat.set_pull(Pull::Down)?;

    let mut heart_was_low = true;

    // TODO this will add small differences, because it does not account for computation time
    // replace with an actual timer?
    // create a timer
    let mut added_ms = 0.0;
    let mut last_added_ms = 0.0;
    let reset_time = 10000.0;

    // last measured peak in ms
    let mut potential_current_peak: Option<f32> = None;
    let mut potential_last_peak: Option<f32> = None;

    // the difference between peaks
    let mut avg_difference = 0.0;

    /*
    info!("it's low");
    led.set_low()?;
    */

    // TODO
    // this code be cleaner, if there was an "initialize" function, where no peak and no average
    // was set yet
    // and then a normal calculate function
    loop {
        // we are using thread::sleep here to make sure the watchdog isn't triggered
        FreeRtos::delay_ms(10);

        added_ms += 10.0;
        added_ms = added_ms % reset_time;

        if heartbeat.is_high() && heart_was_low {
            // set current_peak
            potential_current_peak = Some(added_ms);
            heart_was_low = false;
        }

        if heartbeat.is_low() {
            heart_was_low = true;
        }

        if let Some(current_peak) = potential_current_peak {
            potential_current_peak = None;

            // only the first peak
            if let Some(last_peak) = potential_last_peak {
                // When the time resets, we need a different calculation for current difference
                let difference;
                if added_ms < last_added_ms {
                    difference = current_peak - (last_peak - reset_time);
                } else {
                    difference = current_peak - last_peak;
                }

                // only the first peak
                potential_last_peak = Some(current_peak);

                // only the first difference
                if avg_difference == 0.0 {
                    avg_difference = difference;
                } else {
                    avg_difference = (avg_difference + difference) / 2.0;
                }
            } else {
                potential_last_peak = Some(current_peak);
            }
        }

        // 60  = 1000ms / 1000ms * 60
        // 30  = 1000ms / 2000ms * 60
        // 120 = 1000ms / 500ms  * 60

        if avg_difference > 0.0 {
            info!("The bpm is: {:?}", (1000.0 / avg_difference) * 60.0);
        }
        last_added_ms = added_ms;
    }

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    let ip = connect_wifi(&mut wifi, WIFI_SSID, WIFI_PASS)?;
    info!("IP INFO!!! {:?}", ip);

    // Create thread to receive/send OSC
    // Larger stack size is required (default is 3 KB)
    // You can customize default value by CONFIG_ESP_SYSTEM_EVENT_TASK_STACK_SIZE in sdkconfig
    let port = OSC_PORT.parse::<u16>().unwrap();
    let osc_join_handle = std::thread::Builder::new()
        .stack_size(8192)
        .spawn(move || {
            // put rx for the heart data in here
            let mut osc = Osc::new(ip, port);
            loop {
                if let Err(e) = osc.run() {
                    error!("Failed to run OSC: {e}");
                    break;
                }
            }
        })?;

    // can be used for reading the heart sensor for example.
    // osc_join_handle.join().unwrap();

    // Keep wifi and the server running beyond when main() returns (forever)
    // Do not call this if you ever want to stop or access them later.
    // Otherwise you can either add an infinite loop so the main task
    // never returns, or you can move them to another thread.
    // https://doc.rust-lang.org/stable/core/mem/fn.forget.html
    core::mem::forget(wifi);

    Ok(())
}
