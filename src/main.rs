#![feature(addr_parse_ascii)]

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
use std::net::*;
use std::sync::mpsc;

use log::*;

const WIFI_SSID: &str = env!("OSC_WIFI_SSID");
const WIFI_PASS: &str = env!("OSC_WIFI_PASS");
const OSC_PORT: &str = env!("OSC_PORT");
const OSC_IP: &str = env!("OSC_IP");

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let (tx, rx) = mpsc::channel::<f32>();

    // Setup Wifi
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    let ip = connect_wifi(&mut wifi, WIFI_SSID, WIFI_PASS)?;

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
                let bpm = rosc::OscType::Float(bpm);
                let ip_in_bytes = OSC_IP.as_bytes();
                let ip = Ipv4Addr::parse_ascii(ip_in_bytes).expect("could not convert it to ipv4");
                let addr = SocketAddr::new(IpAddr::V4(ip), port);
                if let Err(e) = osc.run(addr, "/test", bpm) {
                    error!("Failed to run OSC: {e}");
                    break;
                }
                // osc.ping();
            }
        })?;

    let led_pin = peripherals.pins.gpio20;
    let heartbeat_pin = peripherals.pins.gpio3;
    let sen0203_join_handle = std::thread::Builder::new()
        .stack_size(4096)
        .spawn(move || {
            let mut sen0203 =
                Sen0203::new(led_pin, heartbeat_pin).expect("Could not initialize Sen0203");
            loop {
                if let Some(bpm) = sen0203.run() {
                    if let Err(e) = tx.send(bpm) {
                        error!("Failed to send bpm: {e}");
                        break;
                    }
                }
            }
        })?;

    // can be used for reading the heart sensor for example.
    osc_join_handle.join().unwrap();
    sen0203_join_handle.join().unwrap();

    // Keep wifi and the server running beyond when main() returns (forever)
    // Do not call this if you ever want to stop or access them later.
    // Otherwise you can either add an infinite loop so the main task
    // never returns, or you can move them to another thread.
    // https://doc.rust-lang.org/stable/core/mem/fn.forget.html
    core::mem::forget(wifi);

    Ok(())
}
