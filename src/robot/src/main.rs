#![warn(clippy::pedantic)]

mod uart;

use crate::uart::GConf;
use log::{debug, error, info, warn};
use rppal::gpio::{Gpio, Level, OutputPin};
use std::{
    env,
    fmt::Display,
    io::stdin,
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

const FULLSTEPS_PER_REVOLUTION: u32 = 200;

#[derive(Copy, Clone)]
enum Face {
    R,
    L,
    U,
    D,
    F,
    B,
}

/// Configuration for a single TMC2209-controlled motor.
struct TMC2209Config {
    face: Face,
    step_pin: u8,
    dir_pin: u8,
    #[allow(unused)]
    diag_pin: u8,
    #[allow(unused)]
    en_pin: u8,
}

enum Microsteps {
    FullStep = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
    Sixteen = 16,
    ThirtyTwo = 32,
    SixtyFour = 64,
    OneTwentyEight = 128,
    TwoFiftySix = 256,
}

/// Global robot configuration.
struct RobotConfig {
    tmc_2209_configs: [TMC2209Config; 6],
    revolutions_per_second: f64,
    // microsteps: Microsteps,
}

/// Which UART port to use (BCM numbering context).
#[derive(Debug, Copy, Clone)]
enum WhichUart {
    Uart0, // TX: 14, RX: 15 (BCM)
    Uart2, // TX: 0, RX: 1 (BCM)
}

/// Helper for accurate sleep intervals.
struct Ticker {
    now: Instant,
}

impl Default for RobotConfig {
    fn default() -> Self {
        RobotConfig {
            revolutions_per_second: 0.5,
            tmc_2209_configs: [
                TMC2209Config {
                    face: Face::R,
                    step_pin: 13,
                    dir_pin: 19,
                    diag_pin: 0,
                    en_pin: 0,
                },
                TMC2209Config {
                    face: Face::L,
                    step_pin: 20,
                    dir_pin: 21,
                    diag_pin: 0,
                    en_pin: 0,
                },
                TMC2209Config {
                    face: Face::U,
                    step_pin: 17,
                    dir_pin: 27,
                    diag_pin: 0,
                    en_pin: 0,
                },
                TMC2209Config {
                    face: Face::D,
                    step_pin: 5,
                    dir_pin: 6,
                    diag_pin: 0,
                    en_pin: 0,
                },
                TMC2209Config {
                    face: Face::F,
                    step_pin: 16,
                    dir_pin: 26,
                    diag_pin: 0,
                    en_pin: 0,
                },
                TMC2209Config {
                    face: Face::B,
                    step_pin: 2,
                    dir_pin: 3,
                    diag_pin: 0,
                    en_pin: 0,
                },
            ],
        }
    }
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

impl FromStr for Face {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "R" => Ok(Face::R),
            "L" => Ok(Face::L),
            "U" => Ok(Face::U),
            "D" => Ok(Face::D),
            "F" => Ok(Face::F),
            "B" => Ok(Face::B),
            _ => Err(()),
        }
    }
}

fn main() {
    env_logger::init();

    let mut args = env::args();
    let subcommand = args.nth(1);

    let robot_config = RobotConfig::default();

    run_uart_init();

    match subcommand.as_deref() {
        Some("uart-repl") => run_uart_repl(),
        Some("move-seq") => match args.next() {
            Some(seq) => {
                run_move_seq(
                    &robot_config,
                    seq.split(' ').map(str::trim).filter(|v| !v.is_empty()),
                );
            }
            None => {
                eprintln!("Missing move-seq argument");
            }
        },
        Some("motor") => {
            let motor_index = read_num("Enter the motor index: ") as usize;
            run_motor_repl(&robot_config, motor_index);
        }
        other => eprintln!("Unknown or missing subcommand: {other:?}"),
    }
    eprintln!("Exiting");
}

fn run_uart_repl() {
    let which_uart = match read_num("Which UART? (0 or 2) ") {
        0 => WhichUart::Uart0,
        2 => WhichUart::Uart2,
        n => {
            error!(target: "uart_repl", "invalid UART selection: {n} (expected 0 or 2)");
            return;
        }
    };

    eprintln!("register_info: GCONF(reg=0,n=10,RW), GSTAT(reg=1,n=3,R+WC), IFCNT(reg=2,n=8,R)");

    let mut uart = uart::mk_uart(which_uart);

    loop {
        let node_address = read_num("Node address? (0-3) ").try_into().unwrap();
        let register_address = read_num("Register address? (0-127) ").try_into().unwrap();
        let maybe_val = maybe_read_num("Value? (leave blank to read) ");

        if let Some(val) = maybe_val {
            uart::write(&mut uart, node_address, register_address, val);
            eprintln!("Successfully wrote to UART");
        } else {
            let val = uart::read(&mut uart, node_address, register_address);

            match register_address {
                0 => eprintln!(
                    "Read: node_address={node_address} register_address=0(GCONF) val={:?}",
                    uart::GConf::from_bits_retain(val)
                ),
                1 => eprintln!(
                    "Read: node_address={node_address} register_address=1(GSTAT) val={:?}",
                    uart::GStat::from_bits_retain(val)
                ),
                _ => eprintln!(
                    "Read: node_address={node_address} register_address={register_address} raw=0x{val:08x}",
                ),
            }
        }
    }
}

fn run_uart_init() {
    for which_uart in [WhichUart::Uart0, WhichUart::Uart2] {
        let mut uart = uart::mk_uart(which_uart);

        for node_address in 0..3 {
            info!(target: "uart_init", "node_address={node_address} reading initial GCONF");
            let initial_gconf = GConf::from_bits_retain(uart::read(&mut uart, node_address, 0x0));
            let new_gconf = (initial_gconf | GConf::MSTEP_REG_SELECT | GConf::PDN_DISABLE).bits();
            info!(
                target: "uart_init",
                "node_address={node_address} writing GCONF new_bits=0x{new_gconf:08x}",
            );
            uart::write(&mut uart, node_address, 0x0, new_gconf);

            info!(target: "uart_init", "node={node_address} reading initial CHOPCONF");
            let initial_chopconf = uart::read(&mut uart, node_address, 0x6C);
            let new_chopconf = initial_chopconf & !(0b1111 << 24) | (0b1000 << 24);
            info!(
                target: "uart_init",
                "node_address={node_address} writing CHOPCONF new_value=0x{new_chopconf:08x}",
            );
            uart::write(&mut uart, node_address, 0x6C, new_chopconf);
        }
    }
}

fn run_move_seq<'a>(robot_config: &RobotConfig, iter: impl Iterator<Item = &'a str>) {
    let freq = robot_config.revolutions_per_second * f64::from(FULLSTEPS_PER_REVOLUTION);
    let delay = Duration::from_secs(1).div_f64(2.0 * freq);
    info!(
        target: "move_seq",
        "Configuration: freq={freq}rev/s delay={delay:?}",
    );

    let mut step_pins: [OutputPin; 6] =
        std::array::from_fn(|i| mk_output_pin(robot_config.tmc_2209_configs[i].step_pin));
    let mut dir_pins: [OutputPin; 6] =
        std::array::from_fn(|i| mk_output_pin(robot_config.tmc_2209_configs[i].dir_pin));

    for (motor_index, qturns) in iter.map(|s| parse_move(robot_config, s)) {
        info!(
            target: "move_seq",
            "Requested moves: motor_index={motor_index} quarter_turns={qturns}",
        );

        let dir_pin = &mut dir_pins[motor_index];
        let step_pin = &mut step_pins[motor_index];

        let dir_level = if qturns < 0 { Level::Low } else { Level::High };
        dir_pin.write(dir_level);
        debug!(
            target: "move_seq",
            "Set dir level: motor_index={motor_index} dir_level={dir_level}"
        );

        let step_count = qturns.unsigned_abs() * FULLSTEPS_PER_REVOLUTION / 4;
        let mut ticker = Ticker::new();
        for i in 0..step_count {
            if (i % 10) == 0 {
                debug!(
                    target: "move_seq",
                    "Executing {step_count} steps: motor_index={motor_index} 0/{step_count}"
                );
            }
            step_pin.set_high();
            ticker.wait(delay);
            step_pin.set_low();
            ticker.wait(delay);
        }

        info!(
            target: "move_seq",
            "Completed {step_count} steps: motor_index={motor_index}"
        );
    }

    eprintln!("Successfully completed move sequence");
}

fn run_motor_repl(config: &RobotConfig, motor_index: usize) {
    let mut step_pin = mk_output_pin(config.tmc_2209_configs[motor_index].step_pin);
    let steps_per_revolution = f64::from(FULLSTEPS_PER_REVOLUTION);

    eprintln!("1 rev = {steps_per_revolution} steps");
    eprintln!("1 rev/s = {steps_per_revolution} Hz");
    eprintln!(
        "1 ns = {:.2} rev/s (inverse relationship)",
        1_000_000_000.0 / steps_per_revolution
    );
    eprintln!(
        "1 us = {:.2} rev/s (inverse relationship)",
        1_000_000.0 / steps_per_revolution
    );
    loop {
        let mut line = String::new();
        let freq = loop {
            line.clear();
            eprintln!("Enter frequency with units: ");
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
                continue;
            };

            let Ok(v) = rest.trim().parse::<f64>() else {
                continue;
            };

            break match unit {
                "rev/s" => v * steps_per_revolution,
                "Hz" => v,
                "ns" => 1_000_000_000.0 / v,
                "us" => 1_000_000.0 / v,
                _ => unreachable!(),
            };
        };

        run_square_wave(
            &mut step_pin,
            freq,
            Duration::from_secs(4),
        );
    }
}

fn run_square_wave(step: &mut OutputPin, freq: f64, mut dur: Duration) {
    let delay = Duration::from_secs(1).div_f64(2.0 * freq);

    info!(
        target: "square_wave",
        "Configuration: freq={freq}rev/s delay={delay:?}",
    );

    let mut ticker = Ticker::new();
    while !dur.is_zero() {
        step.set_high();
        ticker.wait(delay);
        step.set_low();
        ticker.wait(delay);

        dur = dur.saturating_sub(delay * 2);
    }

    info!(target: "square_wave", "completed run freq_hz={freq:.3}");
}

fn mk_output_pin(gpio: u8) -> OutputPin {
    debug!(target: "gpio", "attempting to configure GPIO pin {gpio}");
    let mut pin = Gpio::new().unwrap().get(gpio).unwrap().into_output_low();
    pin.set_reset_on_drop(false);
    debug!(target: "gpio", "configured GPIO pin {gpio} as output (initial low)");
    pin
}

fn read_num(prompt: impl Display) -> u32 {
    loop {
        if let Some(val) = maybe_read_num(&prompt) {
            return val;
        }
        eprintln!("Try again");
    }
}

fn maybe_read_num(prompt: impl Display) -> Option<u32> {
    let mut line = String::new();
    loop {
        line.clear();
        eprintln!("{prompt}");
        stdin().read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            break None;
        } else if let Ok(v) = line.trim().parse::<u32>() {
            break Some(v);
        }
        eprintln!("Try again");
    }
}

fn parse_move(config: &RobotConfig, mut move_: &str) -> (usize, i32) {
    let qturns = if let Some(rest) = move_.strip_suffix('\'') {
        move_ = rest;
        -1
    } else if let Some(rest) = move_.strip_suffix('2') {
        move_ = rest;
        2
    } else {
        1
    };

    let face_parsed: Face = move_.parse().expect("invalid move: {s}");
    let motor_index = config
        .tmc_2209_configs
        .iter()
        .position(|cfg| cfg.face as u8 == face_parsed as u8)
        .expect("invalid move: {s}");
    (motor_index, qturns)
}
