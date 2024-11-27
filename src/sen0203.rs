use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::{Input, InputPin, Output, OutputPin, Pin, PinDriver},
};
use std::time::Instant;

pub struct Sen0203<'a, S: Pin, T: Pin> {
    led: PinDriver<'a, S, Output>,
    heartbeat: PinDriver<'a, T, Input>,
    heart_was_low: bool,
    last_peak: Instant,
    potential_current_peak: Option<Instant>,
    avg_difference_in_seconds: f32,
}

impl<S, T> Sen0203<'_, S, T>
where
    S: OutputPin,
    T: InputPin,
{
    pub fn new(led_gpio: S, heartbeat_gpio: T) -> anyhow::Result<Self, anyhow::Error> {
        let led = PinDriver::output(led_gpio)?;
        let heartbeat = PinDriver::input(heartbeat_gpio)?;
        let heart_was_low = true;

        // last measured peak in ms
        let potential_current_peak: Option<Instant> = None;
        let last_peak = Instant::now();

        // the difference between peaks
        let avg_difference = 0.0;

        Ok(Self {
            led,
            heartbeat,
            heart_was_low,
            last_peak,
            potential_current_peak,
            avg_difference_in_seconds: avg_difference,
        })
    }

    pub fn run(self: &mut Self) -> Option<f32> {
        FreeRtos::delay_ms(10);

        if self.heartbeat.is_high() && self.heart_was_low {
            // set current_peak
            self.potential_current_peak = Some(Instant::now());
            self.heart_was_low = false;
        }

        if self.heartbeat.is_low() {
            self.heart_was_low = true;
        }

        if let Some(current_peak) = self.potential_current_peak {
            self.potential_current_peak = None;

            // When the time resets, we need a different calculation for current difference
            let difference_in_seconds = current_peak.duration_since(self.last_peak).as_secs_f32();

            self.last_peak = current_peak;

            if difference_in_seconds > 0.2 {
                // only the first valid difference
                if self.avg_difference_in_seconds < 0.1 {
                    self.avg_difference_in_seconds = difference_in_seconds;
                } else {
                    self.avg_difference_in_seconds =
                        (self.avg_difference_in_seconds + difference_in_seconds) / 2.0;
                }
                // can only be 0 < avg
                return Some((1.0 / self.avg_difference_in_seconds) * 60.0);
            }
        }
        None
    }

    pub fn set_led(self: &mut Self, high: bool) -> anyhow::Result<(), anyhow::Error> {
        if high {
            self.led.set_high()?;
        } else {
            self.led.set_low()?;
        }
        Ok(())
    }
}
