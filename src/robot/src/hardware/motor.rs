use std::{
    thread,
    time::{Duration, Instant},
};

use log::debug;
use rppal::gpio::{Gpio, Level, OutputPin};

use crate::hardware::config::MotorConfig;

struct Ticker {
    now: Instant,
}

impl Ticker {
    fn new() -> Self {
        Self {
            now: Instant::now(),
        }
    }

    fn wait(&mut self, delay: Duration) {
        // Advance the expected next time and sleep until that instant.
        self.now += delay;
        thread::sleep(self.now.saturating_duration_since(Instant::now()));
    }
}

pub struct Motor {
    step: OutputPin,
    dir: OutputPin,
}

impl Motor {
    pub fn new(config: &MotorConfig) -> Self {
        fn mk_output_pin(gpio: u8) -> OutputPin {
            debug!(target: "gpio", "attempting to configure GPIO pin {gpio}");
            let mut pin = Gpio::new().unwrap().get(gpio).unwrap().into_output_low();
            pin.set_reset_on_drop(false);
            debug!(target: "gpio", "configured GPIO pin {gpio} as output (initial low)");
            pin
        }

        Self {
            step: mk_output_pin(config.step_pin),
            dir: mk_output_pin(config.dir_pin),
        }
    }

    pub fn turn(&mut self, steps: i32, steps_per_sec: f64) {
        self.dir
            .write(if steps < 0 { Level::Low } else { Level::High });

        let mut ticker = Ticker::new();
        let delay = Duration::from_secs(1).div_f64(2.0 * steps_per_sec);
        for _ in 0..steps.unsigned_abs() {
            self.step.set_high();
            ticker.wait(delay);
            self.step.set_low();
            ticker.wait(delay);
        }
    }
}
