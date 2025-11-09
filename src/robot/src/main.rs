#![warn(clippy::pedantic)]

mod uart;

use crate::uart::GConf;
use rppal::gpio::{Gpio, Level, OutputPin};
use std::{
    env,
    fmt::Display,
    io::stdin,
    thread,
    time::{Duration, Instant},
};

const STEP_PIN: u8 = 13;
const DIR_PIN: u8 = 19;

enum WhichUart {
    Uart0, // TX: 14, RX: 15 (BCM)
    Uart2, // TX: 0, RX: 1 (BCM)
}
use WhichUart::*;
const UARTS: [(WhichUart, u8); 6] = [
    (Uart0, 0),
    (Uart0, 1),
    (Uart0, 2),
    (Uart2, 0),
    (Uart2, 1),
    (Uart2, 2),
];

struct Ticker {
    tick: Instant,
}

impl Ticker {
    fn new() -> Self {
        Self {
            tick: Instant::now(),
        }
    }

    fn wait(&mut self, delay: Duration) {
        self.tick += delay;
        thread::sleep(self.tick.saturating_duration_since(Instant::now()));
    }
}

fn main() {
    let mut args = env::args();
    let subcommand = args.nth(1);
    match subcommand.as_deref() {
        Some("uart") => return run_uart_repl(),
        Some("uart-init") => {
            let mut uart = uart::mk_uart("/dev/ttyAMA0");

            let initial_gconf = GConf::from_bits_retain(uart::read(&mut uart, 0, 0x0));
            uart::write(
                &mut uart,
                0,
                0x0,
                (initial_gconf | GConf::MSTEP_REG_SELECT | GConf::PDN_DISABLE).bits(),
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
        Some("move-seq") => {
            let next_arg = args.next().unwrap();
            return run_move_seq(next_arg.split(" ").map(str::trim).filter(|v| !v.is_empty()));
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

        run_pulse_width_modulation(&mut step, freq, Duration::from_secs(4), spr);
    }
}

fn run_pulse_width_modulation(step: &mut OutputPin, freq: f64, mut dur: Duration, spr: u32) {
    let delay = Duration::from_secs(1).div_f64(2.0 * freq);

    eprintln!("delay = {:?}", delay * 2);
    eprintln!("freq = {:.1} Hz", freq);
    eprintln!("speed = {:.3} rev/s", freq / spr as f64);

    eprint!("Running for {dur:?}...");

    let mut ticker = Ticker::new();
    while !dur.is_zero() {
        step.set_high();
        ticker.wait(delay);
        step.set_low();
        ticker.wait(delay);

        dur = dur.saturating_sub(delay * 2);
    }

    eprintln!(" done.");
}

fn run_uart_repl() {
    let mut uart = uart::mk_uart("/dev/ttyAMA0");
    eprintln!("GCONF = register 0, n = 10, RW");
    eprintln!("GSTAT = register 1, n = 3, R+WC");
    eprintln!("IFCNT = register 2, n = 8, R");
    loop {
        let address = read_num("Address? ") as u8;
        let register = read_num("Register? ") as u8;
        let val = read_num_opt("Value? ");
        if let Some(val) = val {
            eprintln!("Writing {val} to register {register}...");
            uart::write(&mut uart, address, register, val);
            eprintln!("Done.");
        } else {
            eprintln!("Reading from register {register}...");
            let val = uart::read(&mut uart, address, register);
            eprintln!("Done.");

            match register {
                0 => eprintln!("read: {:?}", uart::GConf::from_bits_retain(val)),
                1 => eprintln!("read: {:?}", uart::GStat::from_bits_retain(val)),
                _ => eprintln!("read: {val}"),
            }
        }
    }
}

fn run_move_seq<'a>(iter: impl Iterator<Item = &'a str>) {
    const FREQ: f64 = 6.0 * 200.0;
    // can't be const bc `div_f64` isn't const
    let delay = Duration::from_secs(1).div_f64(2.0 * FREQ);

    // BCM scheme
    // change length to 6 once we have all 6 motors
    const STEP_PINS: [u8; 2] = [13, 20];
    const DIR_PINS: [u8; 2] = [19, 21];


    let iter = iter.map(parse_move);

    let gpio = Gpio::new().unwrap();

    let mut steps = STEP_PINS.map(|i| {
        let mut pin = gpio.get(i).unwrap().into_output_low();
        pin.set_reset_on_drop(false);
        pin
    });
    let mut dirs = DIR_PINS.map(|i| {
        let mut pin = gpio.get(i).unwrap().into_output_low();
        pin.set_reset_on_drop(false);
        pin
    });

    let mut uart0 = uart::mk_uart("/dev/ttyAMA0");
    let mut uart2 = uart::mk_uart("/dev/ttyAMA1");

    for (i, (which_uart, address)) in UARTS.into_iter().enumerate() {
        // remove once we have all 6 motors
        if !(i < 2) {
            continue;
        }

        let uart = match which_uart {
            Uart0 => &mut uart0,
            Uart2 => &mut uart2,
        };

        let mut gconf = GConf::from_bits_retain(uart::read(uart, address, 0x0));
        // TODO: the stepper driver needs a small delay between uart operations, for now i just
        //       sleep for 1ms but eventually this should be integrated into the actual uart code
        thread::sleep(Duration::from_millis(1));
        gconf |= GConf::MSTEP_REG_SELECT | GConf::PDN_DISABLE;
        uart::write(uart, address, 0x0, gconf.bits());
        thread::sleep(Duration::from_millis(1));

        let mut chopconf = uart::read(uart, address, 0x6C);
        thread::sleep(Duration::from_millis(1));
        chopconf = chopconf & !(0b_1111 << 24) | (8 << 24);
        uart::write(uart, address, 0x6C, chopconf);
        thread::sleep(Duration::from_millis(1));
    }

    for (motor_index, qturns) in iter {
        let dir = &mut dirs[motor_index];
        let step = &mut steps[motor_index];

        thread::sleep(Duration::from_millis(500));
        dir.write(if qturns < 0 { Level::Low } else { Level::High });
        let step_count = qturns.unsigned_abs() * 50;
        let mut ticker = Ticker::new();
        for _ in 0..step_count {
            step.set_high();
            ticker.wait(delay);
            step.set_low();
            ticker.wait(delay);
        }
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

fn parse_move(mut s: &str) -> (usize, i32) {
    let qturns = if let Some(rest) = s.strip_suffix("'") {
        s = rest;
        -1
    } else if let Some(rest) = s.strip_suffix("2") {
        s = rest;
        2
    } else {
        1
    };

    let face = match s {
        "R" => 0,
        "L" => 1,
        "U" => 2,
        "D" => 3,
        "F" => 4,
        "B" => 5,
        _ => panic!(),
    };

    (face, qturns)
}
