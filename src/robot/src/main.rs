#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

mod uart;
mod regs;

use clap::{Parser, Subcommand, ValueEnum};
use env_logger::TimestampPrecision;
use log::{LevelFilter, debug, info, warn};
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
const FULLSTEPS_PER_QUARTER: u32 = FULLSTEPS_PER_REVOLUTION / 4;
const NODES_PER_UART: u8 = 3;

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

#[derive(Clone, Copy, Deserialize, Serialize)]
enum Microsteps {
    Fullstep = 8,
    Two = 7,
    Four = 6,
    Eight = 5,
    Sixteen = 4,
    ThirtyTwo = 3,
    SixtyFour = 2,
    OneTwentyEight = 1,
    TwoFiftySix = 0,
}

/// Global robot configuration.
#[derive(Deserialize, Serialize)]
struct RobotConfig {
    tmc_2209_configs: [TMC2209Config; 6],
    revolutions_per_second: f64,
    microsteps: Microsteps,
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
enum Face {
    R,
    L,
    U,
    D,
    F,
    B,
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

impl Microsteps {
    fn mres_bits(self) -> [bool; 4] {
        // 0000 256
        // 0001 128
        // 0010 64
        // 0011 32
        // 0100 16
        // 0101 8
        // 0110 4
        // 0111 2
        // 1000 1
        match self {
            Microsteps::Fullstep => [false, false, false, true],
            Microsteps::Two => [true, true, true, false],
            Microsteps::Four => [false, true, true, false],
            Microsteps::Eight => [true, false, true, false],
            Microsteps::Sixteen => [false, false, true, false],
            Microsteps::ThirtyTwo => [true, true, false, false],
            Microsteps::SixtyFour => [false, true, false, false],
            Microsteps::OneTwentyEight => [true, false, false, false],
            Microsteps::TwoFiftySix => [false, false, false, false],
        }
    }

    fn value(self) -> u32 {
        match self {
            Microsteps::Fullstep => 1,
            Microsteps::Two => 2,
            Microsteps::Four => 4,
            Microsteps::Eight => 8,
            Microsteps::Sixteen => 16,
            Microsteps::ThirtyTwo => 32,
            Microsteps::SixtyFour => 64,
            Microsteps::OneTwentyEight => 128,
            Microsteps::TwoFiftySix => 256,
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The robot configuration file to use, in TOML format.
    #[arg(
        long,
        short = 'c',
        default_value = "robot_conifg.toml",
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
    UartRepl {
        /// Choose Uart0 or Uart2
        which_uart: WhichUart,
    },
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
    /// Initialize UART configuration.
    UartInit,
}

fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(match cli.log_level {
            0 => LevelFilter::Warn,
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        })
        .format_timestamp(Some(TimestampPrecision::Millis))
        .init();

    let robot_config = toml::from_str::<RobotConfig>(
        &std::fs::read_to_string(&cli.robot_config)
            .expect("Failed to read robot configuration file"),
    )
    .expect("Failed to parse robot configuration file");

    uart_init(&robot_config);

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
        Commands::UartInit => {}
    }
    eprintln!("Exiting");
}

fn run_uart_repl(which_uart: WhichUart) {
    eprintln!("register_info: GCONF(reg=0,n=10,RW), GSTAT(reg=1,n=3,R+WC), IFCNT(reg=2,n=8,R)");

    let mut uart = uart::mk_uart(which_uart);

    loop {
        let node_address = read_num(format!("Node address? (0-{NODES_PER_UART}) "))
            .try_into()
            .unwrap();
        let register_address = read_num("Register address? (0-127) ").try_into().unwrap();
        let maybe_val = maybe_read_num("Value? (leave blank to read) ");

        if let Some(val) = maybe_val {
            uart::write(&mut uart, node_address, register_address, val);
            eprintln!("Wrote to UART");
            continue;
        }

        let val = uart::read(&mut uart, node_address, register_address);
        match register_address {
            0 => eprintln!(
                "Read: node_address={node_address} register_address=0(GCONF) val={:?}",
                regs::GCONF::from_bits_retain(val)
            ),
            1 => eprintln!(
                "Read: node_address={node_address} register_address=1(GSTAT) val={:?}",
                regs::GSTAT::from_bits_retain(val)
            ),
            _ => eprintln!(
                "Read: node_address={node_address} register_address={register_address} raw=0x{val:08x}",
            ),
        }
    }
}

fn uart_init(robot_config: &RobotConfig) {
    for which_uart in [WhichUart::Uart0, WhichUart::Uart2] {
        let mut uart = uart::mk_uart(which_uart);
        for node_address in 0..NODES_PER_UART {
            debug!(target: "uart_init", "Initializing: which_uart={which_uart:?} node_address={node_address}");

            //
            // Configure GCONF
            //
            debug!(target: "uart_init", "Reading initial GCONF: node_address={node_address}");
            let initial_gconf =
                regs::GCONF::from_bits(uart::read(&mut uart, node_address, regs::GCONF_REGISTER_ADDRESS))
                    .expect("GCONF has unknown bits set");
            debug!(target: "uart_init", "Read initial GCONF: node_address={node_address} initial_value={initial_gconf:?}");
            let new_gconf = initial_gconf
                .union(regs::GCONF::MSTEP_REG_SELECT)
                .union(regs::GCONF::PDN_DISABLE)
                .union(regs::GCONF::INDEX_OTPW);
            if initial_gconf == new_gconf {
                debug!(target: "uart_init", "GCONF already configured");
            } else {
                debug!(
                    target: "uart_init",
                    "Writing GCONF: node_address={node_address} new_value={new_gconf:?}",
                );
                uart::write(
                    &mut uart,
                    node_address,
                    regs::GCONF_REGISTER_ADDRESS,
                    new_gconf.bits(),
                );
            }

            //
            // Configure CHOPCONF
            //
            debug!(target: "uart_init", "Reading initial CHOPCONF: node_address={node_address}");
            let initial_chopconf = regs::CHOPCONF::from_bits(uart::read(
                &mut uart,
                node_address,
                regs::CHOPCONF_REGISTER_ADDRESS,
            ))
            .expect("CHOPCONF has unknown bits set");
            debug!(target: "uart_init", "Read initial CHOPCONF: node_address={node_address} initial_value={initial_chopconf:?}");
            let [mres0, mres1, mres2, mres3] = robot_config.microsteps.mres_bits();
            let mut new_pwmconf = initial_chopconf;
            new_pwmconf.set(regs::CHOPCONF::MRES0, mres0);
            new_pwmconf.set(regs::CHOPCONF::MRES1, mres1);
            new_pwmconf.set(regs::CHOPCONF::MRES2, mres2);
            new_pwmconf.set(regs::CHOPCONF::MRES3, mres3);
            if new_pwmconf == initial_chopconf {
                debug!(target: "uart_init", "CHOPCONF already configured");
            } else {
                debug!(
                    target: "uart_init",
                    "Writing CHOPCONF: node_address={node_address} new_value={new_pwmconf:?}",
                );
                uart::write(
                    &mut uart,
                    node_address,
                    regs::CHOPCONF_REGISTER_ADDRESS,
                    new_pwmconf.bits(),
                );
            }

            //
            // Configure NODECONF. Note that NODECONF is write-only.
            //
            let nodeconf = regs::NODECONF::empty()
                // Set SENDDELAY to 2. SENDDELAY must be at least 2 in a multi-node system.
                //
                // See page 19 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
                .union(regs::NODECONF::SENDDELAY1);
            debug!(
                target: "uart_init",
                "Writing NODECONF: node_address={node_address} value={nodeconf:?}",
            );
            uart::write(
                &mut uart,
                node_address,
                regs::NODECONF_REGISTER_ADDRESS,
                nodeconf.bits(),
            );

            //
            // Configure PWMCONF.
            //
            debug!(target: "uart_init", "Reading initial PWMCONF: node_address={node_address}");
            let initial_pwmconf = regs::PWMCONF::from_bits(uart::read(
                &mut uart,
                node_address,
                regs::PWMCONF_REGISTER_ADDRESS,
            ))
            .expect("PWMCONF has unknown bits set");
            debug!(target: "uart_init", "Read initial PWMCONF: node_address={node_address} initial_value={initial_pwmconf:?}");
            let new_pwmconf = initial_pwmconf
                // Freewheel mode
                .union(regs::PWMCONF::FREEWHEEL0)
                .difference(regs::PWMCONF::FREEWHEEL1);
            if new_pwmconf == initial_pwmconf {
                debug!(target: "uart_init", "PWMCONF already configured");
            } else {
                debug!(
                    target: "uart_init",
                    "Writing PWMCONF: node_address={node_address} new_value={new_pwmconf:?}",
                );
                uart::write(
                    &mut uart,
                    node_address,
                    regs::PWMCONF_REGISTER_ADDRESS,
                    new_pwmconf.bits(),
                );
            }

            //
            // Configure IHOLD_IRUN. Note that IHOLD_IRUN is write-only.
            //
            let ihold_irun = regs::IHOLD_IRUN::empty()
                // Set IRUN to 31
                .union(regs::IHOLD_IRUN::IRUN0)
                .union(regs::IHOLD_IRUN::IRUN1)
                .union(regs::IHOLD_IRUN::IRUN2)
                .union(regs::IHOLD_IRUN::IRUN3)
                .union(regs::IHOLD_IRUN::IRUN4)
                // Set IHOLDDELAY to 0
                .union(regs::IHOLD_IRUN::IHOLDDELAY0);
            debug!(
                target: "uart_init",
                "Writing IHOLD_IRUN: node_address={node_address} value={ihold_irun:?}",
            );
            uart::write(
                &mut uart,
                node_address,
                regs::IHOLD_IRUN_REGISTER_ADDRESS,
                ihold_irun.bits(),
            );

            debug!(target: "uart_init", "Initialized: which_uart={which_uart:?} node_address={node_address}");
        }
    }
}

fn run_move_seq(robot_config: &RobotConfig, sequence: &str) {
    let freq = robot_config.revolutions_per_second
        * f64::from(robot_config.microsteps.value())
        * f64::from(FULLSTEPS_PER_REVOLUTION);
    let delay = Duration::from_secs(1).div_f64(2.0 * freq);
    info!(
        target: "move_seq",
        "Configuration: freq={freq} delay={delay:?}",
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
            "Requested move {:?}: motor_index={motor_index} quarter_turns={qturns}",
            robot_config.tmc_2209_configs[motor_index].face
        );

        let dir_pin = &mut dir_pins[motor_index];
        let step_pin = &mut step_pins[motor_index];

        let dir_level = if qturns < 0 { Level::Low } else { Level::High };
        dir_pin.write(dir_level);
        debug!(
            target: "move_seq",
            "Set dir level: motor_index={motor_index} dir_level={dir_level}"
        );

        let step_count =
            qturns.unsigned_abs() * robot_config.microsteps.value() * FULLSTEPS_PER_QUARTER;
        let mut ticker = Ticker::new();
        for i in 0..step_count {
            if (i % (10 * qturns.unsigned_abs() * robot_config.microsteps.value())) == 0 {
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
            "Completed move {:?}", robot_config.tmc_2209_configs[motor_index].face
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
        } * f64::from(config.microsteps.value());

        let delay = Duration::from_secs(1).div_f64(2.0 * freq);

        info!(
            target: "motor_repl",
            "Configuration: freq={freq} delay={delay:?}",
        );

        let mut ticker = Ticker::new();
        let mut dur = Duration::from_secs(4);
        while !dur.is_zero() {
            step_pin.set_high();
            ticker.wait(delay);
            step_pin.set_low();
            ticker.wait(delay);

            dur = dur.saturating_sub(delay * 2);
        }

        eprintln!("Completed");
    }
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
