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

/// Global robot configuration.
struct RobotConfig {
    tmc_2209_configs: [TMC2209Config; 6],
    revolutions_per_second: f64,
}

/// Which UART port to use (BCM numbering context).
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
            revolutions_per_second: 3.0,
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
    if log::log_enabled!(log::Level::Info) == false {
        let _ = env_logger::try_init();
    }

    info!("app=startup: main() entered");

    let mut args = env::args();
    let subcommand = args.nth(1);
    debug!(target: "app", "app=cli: received subcommand={subcommand:?}");

    let robot_config = RobotConfig::default();

    match subcommand.as_deref() {
        Some("uart") => {
            info!("app=command: uart - starting UART REPL");
            run_uart_repl();
        }
        Some("uart-init") => {
            info!("app=command: uart-init - initializing UARTs and driver registers");
            run_uart_init();
        }
        Some("move-seq") => match args.next() {
            Some(seq) => {
                info!(target: "app", "app=command: move-seq sequence=\"{seq}\"");
                run_move_seq(
                    robot_config,
                    seq.split(' ').map(str::trim).filter(|v| !v.is_empty()),
                );
            }
            None => {
                error!("app=command move-seq: missing sequence argument");
            }
        },
        Some("motor") => {
            info!("app=command: motor - starting motor REPL");
            let motor_index = read_num("Enter the motor index: ") as usize;
            let microsteps = read_num("Enter number of microsteps: ");
            run_motor_repl(robot_config, motor_index, microsteps);
        }
        other => {
            error!(target: "app", "app=cli: unknown or missing subcommand: {other:?}");
        }
    }

    info!("app=shutdown: main() exiting");
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

    let uart_id = match which_uart {
        WhichUart::Uart0 => 0,
        WhichUart::Uart2 => 2,
    };
    info!(target: "uart_repl", "opened UART{}", uart_id);

    info!(target: "uart_repl", "register_info: GCONF(reg=0,n=10,RW), GSTAT(reg=1,n=3,R+WC), IFCNT(reg=2,n=8,R)");

    let mut uart = uart::mk_uart(which_uart);

    loop {
        let node_address = read_num("Node address? (0-3) ") as u8;
        let register_address = read_num("Register address? (0-127) ") as u8;
        let value_opt = read_num_opt("Value? (leave blank to read) ");

        if let Some(val) = value_opt {
            info!(
                target: "uart_repl",
                "action=write node={node_address} register_address={register_address} value=0x{val:08x}",
            );
            uart::write(&mut uart, node_address, register_address, val);
            info!(
                target: "uart_repl",
                "action=write_complete node={node_address} register_address={register_address}",
            );
        } else {
            info!(
                target: "uart_repl",
                "action=read node={node_address} register_address={register_address}",
            );
            let val = uart::read(&mut uart, node_address, register_address);

            match register_address {
                0 => info!(
                    target: "uart_repl",
                    "read_result node={node_address} register_address=0(GCONF) value={:?}",
                    uart::GConf::from_bits_retain(val)
                ),
                1 => info!(
                    target: "uart_repl",
                    "read_result node={node_address} register_address=1(GSTAT) value={:?}",
                    uart::GStat::from_bits_retain(val)
                ),
                _ => info!(
                    target: "uart_repl",
                    "read_result node={node_address} register_address={register_address} raw=0x{val:08x}",
                ),
            }
        }
    }
}

fn run_uart_init() {
    info!(target: "uart_init", "starting UART initialization for all UARTs");
    for which_uart in [WhichUart::Uart0, WhichUart::Uart2] {
        let uart_idx = match which_uart {
            WhichUart::Uart0 => 0,
            WhichUart::Uart2 => 2,
        };
        info!(target: "uart_init", "initializing UART{uart_idx}");
        let mut uart = uart::mk_uart(which_uart);

        for node_address in 0..3 {
            info!(target: "uart_init", "node={node_address} reading initial GCONF");
            let initial_gconf = GConf::from_bits_retain(uart::read(&mut uart, node_address, 0x0));
            let new_gconf = (initial_gconf | GConf::MSTEP_REG_SELECT | GConf::PDN_DISABLE).bits();
            info!(
                target: "uart_init",
                "node={node_address} writing GCONF new_bits=0x{new_gconf:08x}",
            );
            uart::write(&mut uart, node_address, 0x0, new_gconf);

            info!(target: "uart_init", "node={node_address} reading initial CHOPCONF");
            let initial_chopconf = uart::read(&mut uart, node_address, 0x6C);
            let updated_chopconf = initial_chopconf & !(15 << 24) | (8 << 24);
            info!(
                target: "uart_init",
                "node={node_address} writing CHOPCONF new_value=0x{updated_chopconf:08x}",
            );
            uart::write(&mut uart, node_address, 0x6C, updated_chopconf);
        }
    }
    info!(target: "uart_init", "completed UART initialization");
}

fn run_move_seq<'a>(robot_config: RobotConfig, iter: impl Iterator<Item = &'a str>) {
    info!(target: "move_seq", "starting move sequence");
    let freq = robot_config.revolutions_per_second * FULLSTEPS_PER_REVOLUTION as f64;
    let delay = Duration::from_secs(1).div_f64(2.0 * freq);
    debug!(
        target: "move_seq",
        "computed base_freq={freq} rev/s delay_per_half_period={delay:?}",
    );

    let mut steps: [OutputPin; 6] =
        std::array::from_fn(|i| mk_output_pin(robot_config.tmc_2209_configs[i].step_pin));
    let mut dirs: [OutputPin; 6] =
        std::array::from_fn(|i| mk_output_pin(robot_config.tmc_2209_configs[i].dir_pin));

    // Ensure drivers are initialized before attempting motion.
    run_uart_init();

    for (motor_index, qturns) in iter.map(|s| parse_move(&robot_config, s)) {
        info!(
            target: "move_seq",
            "requested move motor_index={motor_index} quarter_turns={qturns}",
        );

        let dir: &mut OutputPin = &mut dirs[motor_index];
        let step: &mut OutputPin = &mut steps[motor_index];

        let level = if qturns < 0 { Level::Low } else { Level::High };
        dir.write(level);
        debug!(
            target: "move_seq",
            "motor_index={motor_index} set dir level={level:?}",
        );

        let step_count = qturns.unsigned_abs() * FULLSTEPS_PER_REVOLUTION / 4;
        info!(
            target: "move_seq",
            "motor_index={motor_index} executing {step_count} steps",
        );

        let mut ticker = Ticker::new();
        for i in 0..step_count {
            if (i % 10) == 0 {
                debug!(
                    target: "move_seq",
                    "motor_index={motor_index} progress step {i}/{step_count}",
                );
            }
            step.set_high();
            ticker.wait(delay);
            step.set_low();
            ticker.wait(delay);
        }

        info!(
            target: "move_seq",
            "motor_index={motor_index} move complete total_steps={step_count}",
        );
    }

    info!(target: "move_seq", "completed move sequence");
}

fn run_motor_repl(config: RobotConfig, motor_index: usize, microsteps: u32) {
    let mut step_pin = mk_output_pin(config.tmc_2209_configs[motor_index].step_pin);
    let steps_per_revolution = microsteps * FULLSTEPS_PER_REVOLUTION;

    info!(
        target: "motor_repl",
        "motor_repl: motor_index={motor_index} microsteps={microsteps} steps_per_revolution={steps_per_revolution}",
    );
    info!(
        target: "motor_repl",
        "frequency input units accepted: rev/s, Hz, ns, us"
    );

    loop {
        let mut line = String::new();
        let freq = loop {
            line.clear();
            info!(target: "motor_repl", "prompt: enter value with units (e.g. 10Hz, 2rev/s)");
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
                warn!(
                    target: "motor_repl",
                    "input parse failed: no recognized unit in '{}'",
                    line.trim()
                );
                continue;
            };

            let Ok(v) = rest.trim().parse::<f64>() else {
                warn!(
                    target: "motor_repl",
                    "invalid numeric value in input: '{}'",
                    rest.trim()
                );
                continue;
            };

            break match unit {
                "rev/s" => v * steps_per_revolution as f64,
                "Hz" => v,
                "ns" => 1_000_000_000.0 / v,
                "us" => 1_000_000.0 / v,
                _ => unreachable!(),
            };
        };

        info!(
            target: "motor_repl",
            "starting square wave motor_index={motor_index} frequency_hz={freq:.3}",
        );
        run_square_wave(
            &mut step_pin,
            freq,
            Duration::from_secs(4),
            steps_per_revolution,
        );
        info!(
            target: "motor_repl",
            "finished square wave motor_index={motor_index} frequency_hz={freq:.3}",
        );
    }
}

fn run_square_wave(step: &mut OutputPin, freq: f64, mut dur: Duration, steps_per_revolution: u32) {
    let delay = Duration::from_secs(1).div_f64(2.0 * freq);

    info!(
        target: "square_wave",
        "start: freq_hz={freq:.3} half_period={delay:?} duration={dur:?} steps_per_revolution={steps_per_revolution}",
    );
    debug!(
        target: "square_wave",
        "derived revs_per_sec={:.6}",
        freq / steps_per_revolution as f64
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
    info!(target: "gpio", "configured GPIO pin {gpio} as output (initial low)");
    pin
}

fn read_num(prompt: impl Display) -> u32 {
    loop {
        if let Some(v) = read_num_opt(&prompt) {
            debug!(target: "io", "parsed numeric input {v} for prompt '{prompt}'");
            return v;
        }
        warn!(target: "io", "invalid input for prompt '{prompt}'; retrying");
    }
}

fn read_num_opt(prompt: impl Display) -> Option<u32> {
    let mut line = String::new();
    loop {
        line.clear();
        info!(target: "io", "prompting: {prompt}");
        stdin().read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            debug!(target: "io", "received empty response for prompt '{prompt}'");
            break None;
        } else if let Ok(v) = line.trim().parse::<u32>() {
            debug!(target: "io", "successfully parsed {v} from input");
            break Some(v);
        }
        warn!(target: "io", "failed to parse integer from input: '{}'", line.trim());
    }
}

fn parse_move(config: &RobotConfig, mut s: &str) -> (usize, i32) {
    // returns (motor_index, quarter_turns) where quarter_turns is +/-1 or 2
    let qturns = if let Some(rest) = s.strip_suffix('\'') {
        s = rest;
        -1
    } else if let Some(rest) = s.strip_suffix('2') {
        s = rest;
        2
    } else {
        1
    };

    let face_parsed: Face = s.parse().unwrap_or_else(|_| {
        error!(target: "parse_move", "invalid move token '{s}'");
        panic!("invalid move: {s}");
    });
    let motor_index = config
        .tmc_2209_configs
        .iter()
        .position(|cfg| cfg.face as u8 == face_parsed as u8)
        .unwrap_or_else(|| {
            error!(target: "parse_move", "invalid move token '{s}'");
            panic!("invalid move: {s}");
        });

    debug!(
        target: "parse_move",
        "token='{s}' => motor_index={motor_index} quarter_turns={qturns}",
    );
    (motor_index, qturns)
}
