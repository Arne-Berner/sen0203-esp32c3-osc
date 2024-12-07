use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::{Input, InputPin, Output, OutputPin, Pin, PinDriver},
};
use std::time::Instant;

pub struct LED<'a, S: Pin, T: Pin> {
    led: PinDriver<'a, S, Output>,
}

impl<S> LED<'_, S, T>
where
    S: OutputPin,
{
    pub fn new(led_gpio: S) -> anyhow::Result<Self, anyhow::Error> {
        let led = PinDriver::output(led_gpio)?;
        Ok(Self { led })
    }

    // Get this out of here
    pub fn set_led(self: &mut Self, high: bool) -> anyhow::Result<(), anyhow::Error> {
        if high {
            self.led.set_high()?;
        } else {
            self.led.set_low()?;
        }
        Ok(())
    }
}
