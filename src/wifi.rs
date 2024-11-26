use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};

use log::*;
use std::net::Ipv4Addr;

pub fn connect_wifi(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    ssid: &str,
    psk: &str,
) -> anyhow::Result<Ipv4Addr> {
    info!("Wifi started");
    wifi.start()?;

    info!("Wifi scan start");
    let ap_infos = wifi.scan()?;
    info!("after scan");
    let ours = ap_infos.into_iter().find(|a| a.ssid == ssid);
    let channel = if let Some(ours) = ours {
        info!("Found AP {ssid} on channel {}", ours.channel);
        Some(ours.channel)
    } else {
        info!("AP {ssid} not found, go with unknown channel");
        None
    };

    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: psk.try_into().unwrap(),
        channel,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    info!("ssid:{:?}", ssid);
    info!("pwd:{:?}", psk);

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    let ip = wifi.wifi().sta_netif().get_ip_info()?.ip;

    Ok(ip)
}

pub fn deinit(wifi: Box<EspWifi<'static>>) {
    drop(wifi);
    info!("Wifi stopped");
}
