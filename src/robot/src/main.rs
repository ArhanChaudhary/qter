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

const STEPS_PER_REVOLUTION: u32 = 200;

struct TMC2209Config {
    which_uart: WhichUart,
    node_address: u8,
    step_pin: u8,
    dir_pin: u8,
    #[allow(unused)]
    diag_pin: u8,
    #[allow(unused)]
    en_pin: u8,
}

struct TMC2209Configs([TMC2209Config; 6]);

// BCM scheme
// change length to 6 once we have all 6 motors
enum WhichUart {
    Uart0, // TX: 14, RX: 15 (BCM)
    Uart2, // TX: 0, RX: 1 (BCM)
}

struct Ticker {
    now: Instant,
}

fn main() {
    let mut args = env::args();
    let subcommand = args.nth(1);
    let tmc_2209_configs = TMC2209Configs::default();
    match subcommand.as_deref() {
        Some("uart") => run_uart_repl(),
        Some("uart-init") => run_uart_init(),
        Some("move-seq") => {
            let next_arg = args.next().unwrap();
            run_move_seq(
                tmc_2209_configs,
                next_arg.split(" ").map(str::trim).filter(|v| !v.is_empty()),
            );
        }
        Some("motor") => {
            let motor_index = read_num("Enter the motor index: ") as usize;
            let microsteps = read_num("Enter number of microsteps: ");
            run_motor_repl(tmc_2209_configs, motor_index, microsteps);
        }
        _ => eprintln!("Try again."),
    }
}

impl Default for TMC2209Configs {
    fn default() -> Self {
        TMC2209Configs([
            TMC2209Config {
                which_uart: WhichUart::Uart0,
                node_address: 0,
                step_pin: 13,
                dir_pin: 19,

                diag_pin: 0,
                en_pin: 0,
            },
            TMC2209Config {
                which_uart: WhichUart::Uart0,
                node_address: 1,
                step_pin: 20,
                dir_pin: 21,

                diag_pin: 0,
                en_pin: 0,
            },
            TMC2209Config {
                which_uart: WhichUart::Uart0,
                node_address: 2,
                step_pin: 17,
                dir_pin: 27,

                diag_pin: 0,
                en_pin: 0,
            },
            TMC2209Config {
                which_uart: WhichUart::Uart2,
                node_address: 0,
                step_pin: 5,
                dir_pin: 6,

                diag_pin: 0,
                en_pin: 0,
            },
            TMC2209Config {
                which_uart: WhichUart::Uart2,
                node_address: 1,
                step_pin: 16,
                dir_pin: 26,

                diag_pin: 0,
                en_pin: 0,
            },
            TMC2209Config {
                which_uart: WhichUart::Uart2,
                node_address: 2,
                step_pin: 2,
                dir_pin: 3,

                diag_pin: 0,
                en_pin: 0,
            },
        ])
    }
}

impl Ticker {
    fn new() -> Self {
        Self {
            now: Instant::now(),
        }
    }

    fn wait(&mut self, delay: Duration) {
        self.now += delay;
        thread::sleep(self.now.saturating_duration_since(Instant::now()));
    }
}

fn run_uart_repl() {
    let which_uart = match read_num("Which uart? ") {
        0 => WhichUart::Uart0,
        2 => WhichUart::Uart2,
        _ => {
            eprintln!("try again");
            return;
        }
    };
    let mut uart = uart::mk_uart(which_uart);
    eprintln!("GCONF = register 0, n = 10, RW");
    eprintln!("GSTAT = register 1, n = 3, R+WC");
    eprintln!("IFCNT = register 2, n = 8, R");
    loop {
        let node_address = read_num("Node address? ") as u8;
        let register = read_num("Register? ") as u8;
        let val = read_num_opt("Value? ");
        if let Some(val) = val {
            eprintln!("Writing {val} to register {register}...");
            uart::write(&mut uart, node_address, register, val);
            eprintln!("Done.");
        } else {
            eprintln!("Reading from register {register}...");
            let val = uart::read(&mut uart, node_address, register);
            eprintln!("Done.");

            match register {
                0 => eprintln!("read: {:?}", uart::GConf::from_bits_retain(val)),
                1 => eprintln!("read: {:?}", uart::GStat::from_bits_retain(val)),
                _ => eprintln!("read: {val}"),
            }
        }
    }
}

fn run_uart_init() {
    for which_uart in [WhichUart::Uart0, WhichUart::Uart2] {
        let mut uart = uart::mk_uart(which_uart);

        for node_address in 0..3 {
            let initial_gconf = GConf::from_bits_retain(uart::read(&mut uart, node_address, 0x0));
            uart::write(
                &mut uart,
                node_address,
                0x0,
                (initial_gconf | GConf::MSTEP_REG_SELECT | GConf::PDN_DISABLE).bits(),
            );
            let initial_chopconf = uart::read(&mut uart, node_address, 0x6C);
            uart::write(
                &mut uart,
                node_address,
                0x6C,
                initial_chopconf & !(15 << 24) | (8 << 24),
            );
        }
    }
}

fn run_move_seq<'a>(tmc_2209_configs: TMC2209Configs, iter: impl Iterator<Item = &'a str>) {
    const FREQ: f64 = 6.0 * STEPS_PER_REVOLUTION as f64;
    // can't be const bc `div_f64` isn't const
    let delay = Duration::from_secs(1).div_f64(2.0 * FREQ);

    let iter = iter.map(parse_move);

    let mut steps: [OutputPin; 6] =
        std::array::from_fn(|i| mk_output_pin(tmc_2209_configs.0[i].step_pin));
    let mut dirs: [OutputPin; 6] =
        std::array::from_fn(|i| mk_output_pin(tmc_2209_configs.0[i].dir_pin));

    let mut uart0 = uart::mk_uart(WhichUart::Uart0);
    let mut uart2 = uart::mk_uart(WhichUart::Uart2);

    for tmc_2209_config in &tmc_2209_configs.0 {
        let uart = match tmc_2209_config.which_uart {
            WhichUart::Uart0 => &mut uart0,
            WhichUart::Uart2 => &mut uart2,
        };

        let mut gconf =
            GConf::from_bits_retain(uart::read(uart, tmc_2209_config.node_address, 0x0));
        gconf |= GConf::MSTEP_REG_SELECT | GConf::PDN_DISABLE;
        uart::write(uart, tmc_2209_config.node_address, 0x0, gconf.bits());

        let mut chopconf = uart::read(uart, tmc_2209_config.node_address, 0x6C);
        chopconf = chopconf & !(0b_1111 << 24) | (8 << 24);
        uart::write(uart, tmc_2209_config.node_address, 0x6C, chopconf);
    }

    for (motor_index, qturns) in iter {
        let dir = &mut dirs[motor_index];
        let step = &mut steps[motor_index];

        dir.write(if qturns < 0 { Level::Low } else { Level::High });
        let step_count = qturns.unsigned_abs() * STEPS_PER_REVOLUTION / 4;
        let mut ticker = Ticker::new();
        for _ in 0..step_count {
            step.set_high();
            ticker.wait(delay);
            step.set_low();
            ticker.wait(delay);
        }
    }
}

fn run_motor_repl(config: TMC2209Configs, motor_index: usize, microsteps: u32) {
    // let dir_pin = mk_output_pin(config.0[motor_index].dir_pin);
    let mut step_pin = mk_output_pin(config.0[motor_index].step_pin);

    let spr = microsteps * STEPS_PER_REVOLUTION;
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

        run_square_wave(&mut step_pin, freq, Duration::from_secs(4), spr);
    }
}

fn run_square_wave(step: &mut OutputPin, freq: f64, mut dur: Duration, spr: u32) {
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

fn mk_output_pin(gpio: u8) -> OutputPin {
    let mut pin = Gpio::new().unwrap().get(gpio).unwrap().into_output_low();
    pin.set_reset_on_drop(false);
    pin
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
