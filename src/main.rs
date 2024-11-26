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
    let mut button = PinDriver::input(peripherals.pins.gpio3)?;

    button.set_pull(Pull::Down)?;
    let mut led_flag = true;

    loop {
        // we are using thread::sleep here to make sure the watchdog isn't triggered
        FreeRtos::delay_ms(10);
        if button.is_high() {
            info!("it's high");
            led.set_high()?;
        } else {
            info!("it's low");
            led.set_low()?;
        }
    }
    // the heart rate sensor lib will have to take the pin used to receive the data
    // it will look for any changes from 0 to 1 on that pin
    // use the averaged distance between all heart beats
    // this is measured:
    // next: Option = none;
    // if let Some(last_time_of_peak) in the 10 seconds
    // next = time_of_peak - last_time_of_peak;
    // last_time_of_peak = time_of_peak;
    // else
    // last_time_of_peak = time_of_peak
    // if Some(next):
    // if (avg == null)
    // let avg = next;
    // else
    // let avg = (avg+next)/2
    // next = None;
    // where first, second and next are the differences between two beats.
    // use modulo to get every 10 seconds
    // and reset last_time_of_peak to None, when last_time > this_time

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
