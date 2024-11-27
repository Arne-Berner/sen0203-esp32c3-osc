use heartbeatc3::osc::Osc;
use heartbeatc3::wifi::*;

use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};

use log::*;

// osc related
const WIFI_SSID: &str = env!("OSC_WIFI_SSID");
const WIFI_PASS: &str = env!("OSC_WIFI_PASS");
const OSC_WIFI_PORT_STR: &str = env!("OSC_PORT");

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Setup Wifi

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    let ip = connect_wifi(&mut wifi, WIFI_SSID, WIFI_PASS)?;

    // Create thread to receive/send OSC
    // Larger stack size is required (default is 3 KB)
    // You can customize default value by CONFIG_ESP_SYSTEM_EVENT_TASK_STACK_SIZE in sdkconfig
    let port = OSC_WIFI_PORT_STR.parse::<u16>().unwrap();
    let _osc_join_handle = std::thread::Builder::new()
        .stack_size(8192)
        .spawn(move || {
            let mut osc = Osc::new(ip, port);
            loop {
                if let Err(e) = osc.ping() {
                    error!("Failed to run OSC: {e}");
                    break;
                }
            }
        })?;

    core::mem::forget(wifi);

    // Main task no longer needed, free up some memory
    Ok(())
}
