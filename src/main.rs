pub mod osc;
mod sen0203;
pub mod wifi;
use osc::Osc;
use sen0203::*;
use wifi::*;

use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};
use std::sync::mpsc;

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

    let (tx, rx) = mpsc::channel::<f32>();

    let led_pin = peripherals.pins.gpio20;
    let heartbeat_pin = peripherals.pins.gpio3;
    let sen0203_join_handle = std::thread::Builder::new()
        .stack_size(4096)
        .spawn(move || {
            let mut sen0203 =
                Sen0203::new(led_pin, heartbeat_pin).expect("Could not initialize Sen0203");
            loop {
                if let Some(bpm) = sen0203.run() {
                    info!("bpm:{:?}", bpm);
                    tx.send(bpm);
                }
            }
        })?;

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
            let mut osc = Osc::new(ip, port);
            loop {
                let bpm = rx.recv().expect("bpm receive channel hung up");
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
