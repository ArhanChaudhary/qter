#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

use clap::{Parser, Subcommand};
use env_logger::TimestampPrecision;
use interpreter::puzzle_states::{RobotLike, run_robot_server};
use log::{LevelFilter, info, warn};
use qter_core::architectures::{Algorithm, mk_puzzle_definition};
use robot::{
    CUBE3, QterRobot, hardware::{
        FULLSTEPS_PER_REVOLUTION, RobotHandle, Ticker,
        config::{Face, Priority, RobotConfig},
        mk_output_pin, set_prio,
    }
};
use std::{
    convert::Infallible, io::{self, BufReader, stdin}, net::TcpListener, path::PathBuf, sync::Arc, thread, time::{Duration, Instant}
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
    /// Execute a sequence of moves.
    MoveSeq {
        /// The move sequence to execute, e.g. "R U' F2".
        sequence: String,
    },
    /// Run a motor REPL to control a single motor.
    Motor {
        /// The face to control.
        face: Face,
    },
    /// Test latencies at the different options for priority level
    TestPrio { prio: Priority },
    /// Host a server to allow the robot to be remote-controlled
    Server { port: u16 },
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
        Commands::MoveSeq { sequence } => {
            let mut robot_handle = RobotHandle::init(&cli.robot_config);

            robot_handle.queue_move_seq(
                &Algorithm::parse_from_string(Arc::clone(&CUBE3), &sequence)
                    .expect("The algorithm is invalid"),
            );
            robot_handle.await_moves();
        }
        Commands::Motor { face } => {
            let robot_handle = RobotHandle::init(&cli.robot_config);

            run_motor_repl(robot_handle.config(), face);
        }
        Commands::TestPrio { prio } => {
            test_prio(prio);
        }
        Commands::Server { port } => {
            server(port).unwrap();
        },
    }
    println!("Exiting");
}

fn run_motor_repl(config: &RobotConfig, face: Face) {
    let mut step_pin = mk_output_pin(config.motors[face].step_pin);
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
        } * f64::from(config.microstep_resolution.value());

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

fn server(port: u16) -> Result<Infallible, io::Error> {
    let listener = TcpListener::bind(format!("0.0.0.0:{port}"))?;

    // TODO: Better way of getting the config. Maybe use `include_str!`?
    let handle = RobotHandle::init(&PathBuf::from("robot_config.toml"));
    let mut robot = QterRobot::initialize(Arc::clone(&mk_puzzle_definition("3x3").unwrap().perm_group), handle);
    
    loop {
        let (socket, _) = listener.accept()?;

        run_robot_server::<_, QterRobot>(BufReader::new(socket), &mut robot)?;
    }
}
