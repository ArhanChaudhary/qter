use std::{
    mem::MaybeUninit,
    thread,
    time::{Duration, Instant},
};

use log::debug;
use rppal::gpio::{Gpio, Level, OutputPin};

use crate::hardware::config::MotorConfig;

/// Runs `N` blocks with delays concurrently.
///
/// Each element of `iters` is a generator that runs one block. Yielding a
/// `Duration` will wait for that long before that generator is resumed again.
/// Returns when all blocks are complete.
fn run_many<const N: usize>(mut iters: [impl Iterator<Item = Duration>; N]) {
    let now = Instant::now();
    let mut times: [_; N] = core::array::from_fn(|i| iters[i].next().map(|dur| now + dur));

    loop {
        let mut min = None;
        for (i, time) in times.iter_mut().enumerate() {
            let Some(time) = time else {
                continue;
            };
            match min {
                None => min = Some((i, time)),
                Some((_, min_time)) if *time < *min_time => min = Some((i, time)),
                _ => {}
            }
        }

        if let Some((i, next_update)) = min {
            thread::sleep(next_update.saturating_duration_since(Instant::now()));
            let dur = iters[i].next();
            match dur {
                None => times[i] = None,
                Some(dur) => *next_update += dur,
            }
        } else {
            break;
        }
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
        Self::turn_many([self], [steps], [steps_per_sec]);
    }

    pub fn turn_many<const N: usize>(
        selves: [&mut Motor; N],
        steps: [i32; N],
        steps_per_sec: [f64; N],
    ) {
        fn array_zip<T, U, const N: usize>(a: [T; N], b: [U; N]) -> [(T, U); N] {
            let a = a.map(MaybeUninit::new);
            let b = b.map(MaybeUninit::new);
            core::array::from_fn(|i| unsafe { (a[i].assume_init_read(), b[i].assume_init_read()) })
        }

        let state = array_zip(selves, array_zip(steps, steps_per_sec));

        run_many(state.map(|(this, (steps, steps_per_sec))| gen move {
            this.dir
                .write(if steps < 0 { Level::Low } else { Level::High });

            let delay = Duration::from_secs(1).div_f64(2.0 * steps_per_sec);

            for _ in 0..steps {
                this.step.set_high();
                yield delay;
                this.step.set_low();
                yield delay;
            }
        }));
    }
}
