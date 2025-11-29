use std::{
    fmt::Display,
    ops::Add,
    path::Path,
    str::FromStr,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use clap::ValueEnum;
use crossbeam::sync::{Parker, Unparker};
use log::{debug, info};
use qter_core::architectures::Algorithm;
use rppal::gpio::{Gpio, Level, OutputPin};
use serde::{Deserialize, Serialize};
use thread_priority::{
    Error, RealtimeThreadSchedulePolicy, ScheduleParams, ThreadPriority,
    set_thread_priority_and_policy, thread_native_id,
    unix::{ThreadSchedulePolicy, set_current_thread_priority},
};

mod motor_math;
pub mod regs;
pub mod uart;

pub const FULLSTEPS_PER_REVOLUTION: u32 = 200;
pub const FULLSTEPS_PER_QUARTER: u32 = FULLSTEPS_PER_REVOLUTION / 4;
pub const NODES_PER_UART: u8 = 3;

/// Configuration for a single TMC2209-controlled motor.
#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct TMC2209Config {
    face: Face,
    step_pin: u8,
    dir_pin: u8,
}

impl TMC2209Config {
    pub fn face(self) -> Face {
        self.face
    }

    pub fn step_pin(self) -> u8 {
        self.step_pin
    }

    pub fn dir_pin(self) -> u8 {
        self.dir_pin
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum Microsteps {
    Fullstep,
    Two,
    Four,
    Eight,
    Sixteen,
    ThirtyTwo,
    SixtyFour,
    OneTwentyEight,
    TwoFiftySix,
}

#[derive(Clone, Copy, Serialize, Deserialize, ValueEnum)]
pub enum Priority {
    /// Leave the priority as whatever the OS decides it to be
    Default,
    /// Set the priority to the maximum non-real-time priority
    MaxNonRT,
    /// Set the priority to the maximum real-time priority that is also lower than any kernel priority
    RealTime,
}

enum MotorMessage {
    QueueMove(Face, Dir),
    PrevMovesDone(Unparker),
}

/// Global robot configuration.
#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct RobotConfig {
    tmc_2209_configs: [TMC2209Config; 6],
    revolutions_per_second: f64,
    microsteps: Microsteps,
    priority: Priority,
    // enable_pin: u8,
}

impl RobotConfig {
    pub fn tmc_2209_configs(&self) -> [TMC2209Config; 6] {
        self.tmc_2209_configs
    }

    pub fn revolutions_per_second(&self) -> f64 {
        self.revolutions_per_second
    }

    pub fn microsteps(&self) -> Microsteps {
        self.microsteps
    }
}

pub struct RobotHandle {
    motor_thread_handle: mpsc::Sender<MotorMessage>,
    config: RobotConfig,
}

impl RobotHandle {
    /// Initialize the robot such that it is ready for use
    pub fn init(config: &Path) -> RobotHandle {
        let robot_config = toml::from_str::<RobotConfig>(
            &std::fs::read_to_string(config).expect("Failed to read robot configuration file"),
        )
        .expect("Failed to parse robot configuration file");

        uart_init(&robot_config);

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || motor_thread(rx, robot_config));

        RobotHandle {
            motor_thread_handle: tx,
            config: robot_config,
        }
    }

    pub fn config(&self) -> &RobotConfig {
        &self.config
    }

    /// Queue a sequence of moves to be performed by the robot
    pub fn queue_move_seq(&mut self, alg: &Algorithm) {
        for moove in alg.move_seq_iter() {
            let (face, dir) = parse_move(moove);

            self.motor_thread_handle
                .send(MotorMessage::QueueMove(face, dir))
                .unwrap();
        }
    }

    /// Wait for all moves in the queue to be performed
    pub fn await_moves(&mut self) {
        let parker = Parker::new();

        self.motor_thread_handle
            .send(MotorMessage::PrevMovesDone(parker.unparker().clone()))
            .unwrap();

        parker.park();
    }
}

/// Which UART port to use (BCM numbering context).
#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum WhichUart {
    Uart0, // TX: 14, RX: 15 (BCM)
    Uart4, // TX: 8, RX: 9 (BCM)
}

/// Helper for accurate sleep intervals.
pub struct Ticker {
    now: Instant,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Face {
    R,
    L,
    U,
    D,
    F,
    B,
}

impl Ticker {
    pub fn new() -> Self {
        Self {
            now: Instant::now(),
        }
    }

    pub fn wait(&mut self, delay: Duration) {
        // Advance the expected next time and sleep until that instant.
        self.now += delay;
        thread::sleep(self.now.saturating_duration_since(Instant::now()));
    }
}

impl Default for Ticker {
    fn default() -> Self {
        Self::new()
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

    pub fn value(self) -> u32 {
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

fn motor_thread(rx: mpsc::Receiver<MotorMessage>, robot_config: RobotConfig) {
    // TODO: State machine for collapsing commutative moves
    // TODO: Motor acceleration curves

    set_prio(robot_config.priority);

    let freq = robot_config.revolutions_per_second()
        * f64::from(robot_config.microsteps().value())
        * f64::from(FULLSTEPS_PER_REVOLUTION);
    let delay = Duration::from_secs(1).div_f64(2.0 * freq);
    info!(
        target: "move_seq",
        "Configuration: freq={freq} delay={delay:?}",
    );

    let mut step_pins: [OutputPin; 6] =
        std::array::from_fn(|i| mk_output_pin(robot_config.tmc_2209_configs()[i].step_pin()));
    let mut dir_pins: [OutputPin; 6] =
        std::array::from_fn(|i| mk_output_pin(robot_config.tmc_2209_configs()[i].dir_pin()));

    for (face, dir) in rx.iter().filter_map(|v| match v {
        MotorMessage::QueueMove(face, dir) => Some((face, dir)),
        MotorMessage::PrevMovesDone(unparker) => {
            unparker.unpark();
            None
        }
    }) {
        // loop {
        let motor_index = robot_config
            .tmc_2209_configs()
            .iter()
            .position(|cfg| cfg.face() as u8 == face as u8)
            .expect("invalid move: {s}");

        info!(
            target: "move_seq",
            "Requested move {:?}: motor_index={motor_index} direction={dir}",
            robot_config.tmc_2209_configs()[motor_index].face()
        );

        let dir_pin = &mut dir_pins[motor_index];
        let step_pin = &mut step_pins[motor_index];

        let qturns = dir.qturns();

        let dir_level = if qturns < 0 { Level::Low } else { Level::High };
        dir_pin.write(dir_level);
        debug!(
            target: "move_seq",
            "Set dir level: motor_index={motor_index} dir_level={dir_level}"
        );

        let step_count =
            qturns.unsigned_abs() * robot_config.microsteps().value() * FULLSTEPS_PER_QUARTER;
        let mut ticker = Ticker::new();
        for i in 0..step_count {
            if (i % (10 * qturns.unsigned_abs() * robot_config.microsteps().value())) == 0 {
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
            "Completed move {:?}", robot_config.tmc_2209_configs()[motor_index].face()
        );
        // }
    }

    println!("Completed move sequence");
}

pub fn set_prio(prio: Priority) {
    let res = match prio {
        // Do nothing
        Priority::Default => return,
        // Set niceness to the maximum (-20)
        Priority::MaxNonRT => set_current_thread_priority(ThreadPriority::Max),
        // Set a real-time priority. 80 is above interrupt handlers but below critical kernel functionalities
        // https://shuhaowu.com/blog/2022/04-linux-rt-appdev-part4.html
        Priority::RealTime => set_thread_priority_and_policy(
            thread_native_id(),
            ThreadPriority::from_posix(ScheduleParams { sched_priority: 80 }),
            ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo),
        ),
    };

    if let Err(e) = res {
        if matches!(e, Error::OS(13)) || matches!(e, Error::OS(1)) {
            panic!(
                "{e} — You need to configure your system such that userspace applications have permission to raise their priorities (unless you're not on unix in which case idk what that error code means)"
            );
        } else {
            panic!("{e}");
        }
    }
}

pub fn uart_init(robot_config: &RobotConfig) {
    for which_uart in [WhichUart::Uart0, WhichUart::Uart4] {
        let mut uart = uart::mk_uart(which_uart);
        for node_address in 0..NODES_PER_UART {
            debug!(target: "uart_init", "Initializing: which_uart={which_uart:?} node_address={node_address}");

            //
            // Configure GCONF
            //
            debug!(target: "uart_init", "Reading initial GCONF: node_address={node_address}");
            let initial_gconf = regs::GCONF::from_bits(uart::read(
                &mut uart,
                node_address,
                regs::GCONF_REGISTER_ADDRESS,
            ))
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
                // Set IHOLDDELAY to 1
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

pub fn estop(robot_config: &RobotConfig) {}

pub fn mk_output_pin(gpio: u8) -> OutputPin {
    debug!(target: "gpio", "attempting to configure GPIO pin {gpio}");
    let mut pin = Gpio::new().unwrap().get(gpio).unwrap().into_output_low();
    pin.set_reset_on_drop(false);
    debug!(target: "gpio", "configured GPIO pin {gpio} as output (initial low)");
    pin
}

#[derive(Debug, Clone, Copy)]
enum Dir {
    Normal,
    Double,
    Prime,
}

impl Dir {
    fn qturns(self) -> i32 {
        match self {
            Dir::Normal => 1,
            Dir::Double => 2,
            Dir::Prime => -1,
        }
    }
}

impl Add<Dir> for Dir {
    type Output = Option<Dir>;

    fn add(self, rhs: Dir) -> Self::Output {
        match (self, rhs) {
            (Dir::Normal, Dir::Prime) => None,
            (Dir::Prime, Dir::Normal) => None,
            (Dir::Double, Dir::Double) => None,
            (Dir::Double, Dir::Prime) => Some(Dir::Normal),
            (Dir::Prime, Dir::Double) => Some(Dir::Normal),
            (Dir::Normal, Dir::Normal) => Some(Dir::Double),
            (Dir::Prime, Dir::Prime) => Some(Dir::Double),
            (Dir::Normal, Dir::Double) => Some(Dir::Prime),
            (Dir::Double, Dir::Normal) => Some(Dir::Prime),
        }
    }
}

impl Display for Dir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dir::Normal => f.write_str("Normal"),
            Dir::Double => f.write_str("Double"),
            Dir::Prime => f.write_str("Prime"),
        }
    }
}

fn parse_move(mut move_: &str) -> (Face, Dir) {
    let dir = if let Some(rest) = move_.strip_suffix('\'') {
        move_ = rest;
        Dir::Prime
    } else if let Some(rest) = move_.strip_suffix('2') {
        move_ = rest;
        Dir::Double
    } else {
        Dir::Normal
    };

    let face_parsed: Face = move_.parse().expect("invalid move: {s}");
    (face_parsed, dir)
}
