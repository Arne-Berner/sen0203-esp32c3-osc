use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::{Input, InputPin, Output, OutputPin, Pin, PinDriver},
};
use std::time::Instant;

const HIGH_BPM_CUTOFF: f32 = 300.0;
const LOW_BPM_CUTOFF: f32 = 30.0;

pub struct Sen0203<'a, T: Pin> {
    heartbeat: PinDriver<'a, T, Input>,
    heart_was_low: bool,
    last_peak: Instant,
    // Not a real average, but an approximation, that fits this usecase more
    avg_beats_per_second: f32,
}

impl<T> Sen0203<'_, T>
where
    T: InputPin,
{
    pub fn new(heartbeat_gpio: T) -> anyhow::Result<Self, anyhow::Error> {
        let heartbeat = PinDriver::input(heartbeat_gpio)?;
        let heart_was_low = true;

        // last measured peak in ms
        let last_peak = Instant::now();

        // the difference between peaks
        let avg_beats_per_second = 1.0;

        Ok(Self {
            heartbeat,
            heart_was_low,
            last_peak,
            avg_beats_per_second,
        })
    }

    pub fn run(self: &mut Self) -> Option<f32> {
        FreeRtos::delay_ms(10);

        if self.heart_was_low {
            if self.heartbeat.is_high() {
                self.heart_was_low = false;
                if let Some(avg) = calculate_fake_avg(){
                    return Some(avg);
                }
            }
        } else {
            if self.heartbeat.is_low() {
                self.heart_was_low = true;
            }
        }
        None
    }

    fn calculate_fake_avg(self: &mut Self) -> Option<f32>{
                    let current_peak = Instant::now();
                    let difference_in_seconds = current_peak.duration_since(self.last_peak).as_secs_f32();

                    if difference_in_seconds <= 0 {
                        return None;
                    }

                    self.last_peak = current_peak;

                    let beats_per_second = 1.0 / difference_in_seconds;
                    if LOW_BPM_CUTOFF < beats_per_second < HIGH_BPM_CUTOFF {
                        self.avg_beats_per_second =
                            (self.avg_beats_per_second + beats_per_second) / 2.0;
                        // can only be 0 < avg
                        return Some(beats_per_second * 60.0);
                    }

    }
}

