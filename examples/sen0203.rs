use esp_idf_svc::hal::prelude::Peripherals;
use heartbeatc3::sen0203::*;

use log::*;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let led_pin = peripherals.pins.gpio20;
    let heartbeat_pin = peripherals.pins.gpio3;
    let mut sen0203 = Sen0203::new(led_pin, heartbeat_pin)?;
    loop {
        if let Some(bpm) = sen0203.run() {
            info!("bpm:{:?}", bpm);
        }
    }
}
