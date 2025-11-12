#![warn(clippy::pedantic)]

mod uart;

use crate::uart::GConf;
use clap::{Parser, Subcommand, ValueEnum};
use log::{debug, info, warn};
use rppal::gpio::{Gpio, Level, OutputPin};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    io::stdin,
    path::PathBuf,
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

const FULLSTEPS_PER_REVOLUTION: u32 = 200;

#[derive(Copy, Clone, Serialize, Deserialize)]
enum Face {
    R,
    L,
    U,
    D,
    F,
    B,
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

/// Configuration for a single TMC2209-controlled motor.
#[derive(Deserialize, Serialize)]
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
#[derive(Deserialize, Serialize)]
struct RobotConfig {
    tmc_2209_configs: [TMC2209Config; 6],
    revolutions_per_second: f64,
    // microsteps: Microsteps,
    // enable_pin: u8,
}

/// Which UART port to use (BCM numbering context).
#[derive(Debug, Copy, Clone, ValueEnum)]
enum WhichUart {
    Uart0, // TX: 14, RX: 15 (BCM)
    Uart2, // TX: 0, RX: 1 (BCM)
}

/// Helper for accurate sleep intervals.
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

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The robot configuration file to use, in TOML format.
    #[arg(
        short,
        long,
        default_missing_value = "robot_conifg.toml",
        value_name = "ROBOT_CONFIG"
    )]
    robot_config: PathBuf,

    /// Increase logging verbosity (can be repeated)
    #[arg(short, long, action = clap::ArgAction::Count)]
    log_level: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a UART REPL to read/write registers.
    UartRepl { which_uart: WhichUart },
    /// Execute a sequence of moves.
    MoveSeq {
        /// The move sequence to execute, e.g. "R U' F2".
        sequence: String,
    },
    /// Run a motor REPL to control a single motor.
    Motor {
        /// The motor index to control (0-5).
        motor_index: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(match cli.log_level {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            _ => log::LevelFilter::Debug,
        })
        .init();

    let robot_config = toml::from_str::<RobotConfig>(
        &std::fs::read_to_string(&cli.robot_config)
            .expect("Failed to read robot configuration file"),
    )
    .expect("Failed to parse robot configuration file");

    run_uart_init();

    match cli.command {
        Commands::UartRepl { which_uart } => {
            run_uart_repl(which_uart);
        }
        Commands::MoveSeq { sequence } => {
            run_move_seq(&robot_config, &sequence);
        }
        Commands::Motor { motor_index } => {
            run_motor_repl(&robot_config, motor_index);
        }
    }
    eprintln!("Exiting");
}

fn run_uart_repl(which_uart: WhichUart) {
    eprintln!("register_info: GCONF(reg=0,n=10,RW), GSTAT(reg=1,n=3,R+WC), IFCNT(reg=2,n=8,R)");

    let mut uart = uart::mk_uart(which_uart);

    loop {
        let node_address = read_num("Node address? (0-3) ").try_into().unwrap();
        let register_address = read_num("Register address? (0-127) ").try_into().unwrap();
        let maybe_val = maybe_read_num("Value? (leave blank to read) ");

        if let Some(val) = maybe_val {
            uart::write(&mut uart, node_address, register_address, val);
            eprintln!("Wrote to UART");
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
            info!(target: "uart_init", "Reading initial GCONF: node_address={node_address}");
            let initial_gconf = GConf::from_bits_retain(uart::read(&mut uart, node_address, 0x0));
            let new_gconf = (initial_gconf | GConf::MSTEP_REG_SELECT | GConf::PDN_DISABLE).bits();
            info!(
                target: "uart_init",
                "Writing GCONF: node_address={node_address} new_value=0x{new_gconf:08x}",
            );
            uart::write(&mut uart, node_address, 0x0, new_gconf);

            info!(target: "uart_init", "reading initial CHOPCONF: node_address={node_address}");
            let initial_chopconf = uart::read(&mut uart, node_address, 0x6C);
            let new_chopconf = initial_chopconf & !(0b1111 << 24) | (0b1000 << 24);
            info!(
                target: "uart_init",
                "Writing CHOPCONF: node_address={node_address} new_value=0x{new_chopconf:08x}",
            );
            uart::write(&mut uart, node_address, 0x6C, new_chopconf);
        }
    }
}

fn run_move_seq(robot_config: &RobotConfig, sequence: &str) {
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

    for (motor_index, qturns) in sequence
        .split(' ')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|s| parse_move(robot_config, s))
    {
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
                    "Executing {step_count} steps: motor_index={motor_index} {i}/{step_count}"
                );
            }
            step_pin.set_high();
            ticker.wait(delay);
            step_pin.set_low();
            ticker.wait(delay);
        }

        info!(
            target: "move_seq",
            "Completed {step_count} steps: motor_index={motor_index} {step_count}/{step_count}"
        );
    }

    eprintln!("Completed move sequence");
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

        run_square_wave(&mut step_pin, freq, Duration::from_secs(4));
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

    info!(target: "square_wave", "Completed square wave");
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
