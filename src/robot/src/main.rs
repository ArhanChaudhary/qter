mod uart;

use std::{
    env,
    fmt::Display,
    io::stdin,
    time::{Duration, Instant},
};

use rppal::gpio::{Gpio, OutputPin};

use crate::uart::GConf;

const SPR: u32 = 200;

const STEP_PIN: u8 = 17; // 11 in board scheme
const DIR_PIN: u8 = 27; // 13 in board scheme

struct Delayer {
    now: Instant,
}

impl Delayer {
    fn new() -> Self {
        Self {
            now: Instant::now(),
        }
    }

    fn wait(&mut self, delay: Duration) {
        // fn sleep_until(instant: Instant) {
        //     let x = instant.saturating_duration_since(Instant::now());
        //     if x.is_zero() {
        //         eprintln!("bad");
        //     }
        //     thread::sleep(x);
        // }

        fn sleep_until(instant: Instant) {
            if Instant::now() < instant {
                while Instant::now() < instant {
                    core::hint::spin_loop();
                }
            } else {
                // eprintln!("bad");
            }
        }

        self.now = self.now + delay;
        sleep_until(self.now);
    }
}

fn read_num(prompt: impl Display) -> u32 {
    loop {
        if let Some(v) = read_num_opt(&prompt) {
            break v;
        }
        eprintln!("try again");
    }
}

fn read_num_opt(prompt: impl Display) -> Option<u32> {
    let mut line = String::new();
    loop {
        line.clear();
        eprint!("{prompt}");
        stdin().read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            break None;
        } else if let Ok(v) = line.trim().parse::<u32>() {
            break Some(v);
        }
        eprintln!("try again");
    }
}

fn main() {
    let mut args = env::args();
    let subcommand = args.nth(1);
    match subcommand.as_deref() {
        Some("uart") => return uart_main(),
        Some("uart-init") => {
            let mut uart = uart::mk_uart("/dev/ttyAMA0");

            let initial_gconf = GConf::from_bits_retain(uart::read(&mut uart, 0, 0x0));
            uart::write(
                &mut uart,
                0,
                0x0,
                (initial_gconf | GConf::MSTEP_REG_SELECT).bits(),
            );

            let initial_chopconf = uart::read(&mut uart, 0, 0x6C);
            uart::write(
                &mut uart,
                0,
                0x6C,
                initial_chopconf & !(15 << 24) | (8 << 24),
            );

            return;
        }
        _ => {}
    }

    let gpio = Gpio::new().unwrap();
    let mut dir = gpio.get(DIR_PIN).unwrap().into_output_low();
    let mut step = gpio.get(STEP_PIN).unwrap().into_output_low();
    dir.set_reset_on_drop(false);
    step.set_reset_on_drop(false);

    let ms = read_num("Enter number of microsteps: ");
    let spr = ms * 200;
    eprintln!("1 rev = {spr} steps");
    eprintln!("1 rev/s = {spr} Hz");
    eprintln!(
        "1 ns = {:.2} rev/s (inverse relationship)",
        1_000_000_000.0 / spr as f64
    );
    eprintln!(
        "1 us = {:.2} rev/s (inverse relationship)",
        1_000_000.0 / spr as f64
    );

    loop {
        let mut line = String::new();
        let freq = loop {
            line.clear();
            eprint!("Enter value with units: ");
            stdin().read_line(&mut line).unwrap();
            line.make_ascii_lowercase();

            let (rest, unit) = if let Some(rest) = line.trim().strip_suffix("rev/s") {
                (rest, "rev/s")
            } else if let Some(rest) = line.trim().strip_suffix("hz") {
                (rest, "Hz")
            } else if let Some(rest) = line.trim().strip_suffix("ns") {
                (rest, "ns")
            } else if let Some(rest) = line.trim().strip_suffix("us") {
                (rest, "us")
            } else {
                eprintln!("try again");
                continue;
            };

            let Ok(v) = rest.trim().parse::<f64>() else {
                eprintln!("try again");
                continue;
            };

            break match unit {
                "rev/s" => v * spr as f64,
                "Hz" => v,
                "ns" => 1_000_000_000.0 / v,
                "us" => 1_000_000.0 / v,
                _ => unreachable!(),
            };
        };

        run2(&mut step, freq, Duration::from_secs(4), spr);

        // let mut freq_accum = 0.0;
        // loop {
        //     freq_accum += 100.0;
        //     if freq_accum >= freq {
        //         run(&mut step, freq, Duration::from_secs(4), spr);
        //         break;
        //     } else {
        //         run(&mut step, freq_accum, Duration::from_secs_f32(0.1), spr);
        //     }
        // }
    }
}

fn run(step: &mut OutputPin, freq: f64, mut dur: Duration, spr: u32) {
    let delay = Duration::from_secs(1).div_f64(2.0 * freq);

    eprintln!("delay = {:?}", delay * 2);
    eprintln!("freq = {:.1} Hz", freq);
    eprintln!("speed = {:.3} rev/s", freq / spr as f64);

    eprint!("Running for {dur:?}...");

    let mut delayer = Delayer::new();
    while !dur.is_zero() {
        step.set_high();
        delayer.wait(delay);
        step.set_low();
        delayer.wait(delay);

        dur = dur.saturating_sub(delay * 2);
    }

    eprintln!(" done.");
}

fn run2(step: &mut OutputPin, freq: f64, mut dur: Duration, spr: u32) {
    let delay = Duration::from_secs(1).div_f64(2.0 * freq);

    eprintln!("delay = {:?}", delay * 2);
    eprintln!("freq = {:.1} Hz", freq);
    eprintln!("speed = {:.3} rev/s", freq / spr as f64);

    eprint!("Spinning up...");

    let mut freq_accum = f64::min(500.0, freq);
    let mut delayer = Delayer::new();

    loop {
        let delay = Duration::from_secs(1).div_f64(2.0 * freq_accum);

        step.set_high();
        delayer.wait(delay);
        step.set_low();
        delayer.wait(delay);

        let dt = delay * 2;
        freq_accum += 500.0 * dt.as_secs_f64();
        if freq_accum > freq {
            break;
        }
    }

    eprintln!(" done.");

    eprint!("Running for {dur:?}...");

    while !dur.is_zero() {
        step.set_high();
        delayer.wait(delay);
        step.set_low();
        delayer.wait(delay);

        let dt = delay * 2;
        dur = dur.saturating_sub(dt);
    }

    eprintln!(" done.");
}

fn uart_main() {
    let mut uart = uart::mk_uart("/dev/ttyAMA0");
    eprintln!("GCONF = register 0, n = 10, RW");
    eprintln!("GSTAT = register 1, n = 3, R+WC");
    eprintln!("IFCNT = register 2, n = 8, R");
    loop {
        let register = read_num("Register? ") as u8;
        let val = read_num_opt("Value? ");
        if let Some(val) = val {
            eprintln!("Writing {val} to register {register}...");
            uart::write(&mut uart, 0, register, val);
            eprintln!("Done.");
        } else {
            eprintln!("Reading from register {register}...");
            let val = uart::read(&mut uart, 0, register);
            eprintln!("Done.");

            match register {
                0 => eprintln!("read: {:?}", uart::GConf::from_bits_retain(val)),
                1 => eprintln!("read: {:?}", uart::GStat::from_bits_retain(val)),
                _ => eprintln!("read: {val}"),
            }
        }
    }
}
