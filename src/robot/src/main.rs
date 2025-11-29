#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

use clap::{Parser, Subcommand};
use env_logger::TimestampPrecision;
use log::{LevelFilter, info, warn};
use qter_core::architectures::Algorithm;
use robot::{
    CUBE3,
    hardware::{
        FULLSTEPS_PER_REVOLUTION, NODES_PER_UART, Priority, RobotConfig, RobotHandle, Ticker,
        WhichUart, mk_output_pin, regs, set_prio, uart,
    },
};
use std::{
    fmt::Display,
    io::stdin,
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The robot configuration file to use, in TOML format.
    #[arg(long, short = 'c', default_value = "robot_config.toml")]
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
    /// Test latencies at the different options for priority level
    TestPrio { prio: Priority },
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

    match cli.command {
        Commands::UartRepl { which_uart } => {
            run_uart_repl(which_uart);
        }
        Commands::MoveSeq { sequence } => {
            let mut robot_handle = RobotHandle::init(&cli.robot_config);

            robot_handle.queue_move_seq(
                &Algorithm::parse_from_string(Arc::clone(&CUBE3), &sequence)
                    .expect("The algorithm is invalid"),
            );
            robot_handle.await_moves();
        }
        Commands::Motor { motor_index } => {
            let robot_handle = RobotHandle::init(&cli.robot_config);

            run_motor_repl(robot_handle.config(), motor_index);
        }
        Commands::UartInit => {}
        Commands::TestPrio { prio } => {
            test_prio(prio);
        }
    }
    println!("Exiting");
}

fn run_uart_repl(which_uart: WhichUart) {
    println!("register_info: GCONF(reg=0,n=10,RW), GSTAT(reg=1,n=3,R+WC), IFCNT(reg=2,n=8,R)");

    let mut uart = uart::mk_uart(which_uart);

    loop {
        let node_address = read_num(format!("Node address? (0-{NODES_PER_UART}) "))
            .try_into()
            .unwrap();
        let register_address = read_num("Register address? (0-127) ").try_into().unwrap();
        let maybe_val = maybe_read_num("Value? (leave blank to read) ");

        if let Some(val) = maybe_val {
            uart::write(&mut uart, node_address, register_address, val);
            println!("Wrote to UART");
            continue;
        }

        let val = uart::read(&mut uart, node_address, register_address);
        match register_address {
            0 => println!(
                "Read: node_address={node_address} register_address=0(GCONF) val={:?}",
                regs::GCONF::from_bits_retain(val)
            ),
            1 => println!(
                "Read: node_address={node_address} register_address=1(GSTAT) val={:?}",
                regs::GSTAT::from_bits_retain(val)
            ),
            _ => println!(
                "Read: node_address={node_address} register_address={register_address} raw=0x{val:08x}",
            ),
        }
    }
}

fn run_motor_repl(config: &RobotConfig, motor_index: usize) {
    let mut step_pin = mk_output_pin(config.tmc_2209_configs()[motor_index].step_pin());
    let steps_per_revolution = f64::from(FULLSTEPS_PER_REVOLUTION);

    println!("1 rev = {steps_per_revolution} steps");
    println!("1 rev/s = {steps_per_revolution} Hz");
    println!(
        "1 ns = {:.2} rev/s (inverse relationship)",
        1_000_000_000.0 / steps_per_revolution
    );
    println!(
        "1 us = {:.2} rev/s (inverse relationship)",
        1_000_000.0 / steps_per_revolution
    );
    loop {
        let mut line = String::new();
        let freq = loop {
            line.clear();
            println!("Enter frequency with units: ");
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
        } * f64::from(config.microsteps().value());

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

        println!("Completed");
    }
}

fn read_num(prompt: impl Display) -> u32 {
    loop {
        if let Some(val) = maybe_read_num(&prompt) {
            return val;
        }
        println!("Try again");
    }
}

fn maybe_read_num(prompt: impl Display) -> Option<u32> {
    let mut line = String::new();
    loop {
        line.clear();
        println!("{prompt}");
        stdin().read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            break None;
        } else if let Ok(v) = line.trim().parse::<u32>() {
            break Some(v);
        }
        println!("Try again");
    }
}

#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_precision_loss)]
fn test_prio(prio: Priority) {
    const SAMPLES: usize = 2048;

    set_prio(prio);
    println!("PID: {}", std::process::id());

    loop {
        let mut latencies = Vec::<i128>::with_capacity(SAMPLES);

        for _ in 0..SAMPLES {
            let before = Instant::now();
            thread::sleep(Duration::from_millis(1));
            let after = Instant::now();

            let time = after - before;
            let nanos = time.as_nanos() as i128;

            let wrongness = nanos - 1_000_000;
            latencies.push(wrongness / 1000);
        }

        latencies.sort_unstable();

        println!("M ≈ {}μs", latencies[SAMPLES / 2]);
        println!(
            "IQR ≈ {}μs",
            (latencies[SAMPLES * 3 / 4] - latencies[SAMPLES / 4])
        );
        println!("Top 5 = {:?}", &latencies[SAMPLES - 5..SAMPLES]);
        println!();
    }
}
