//! A Rust port of the [Movecount Coefficient Calculator](https://trangium.github.io/MovecountCoefficient/)
//! adapted with permission.

// Very blatantly copy pasted from a single pass of AI transpilation

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Location {
    Home,
    Top,
    Bottom,
    UFlick,
    DFlick,
    FFlick,
    SFlick,
    EFlick,
    MFlick,
    E,
    M,
    EidoFlick,
    LeftDb,
    RightDb,
    LeftU,
    RightU,
    RDown,
}

#[derive(Debug, Clone)]
pub struct Finger {
    last_move_time: f64,
    location: Location,
}

impl Finger {
    fn new() -> Self {
        Self {
            last_move_time: -1.0,
            location: Location::Home,
        }
    }
}

#[derive(Debug)]
pub struct HandState {
    thumb: Finger,
    index: Finger,
    middle: Finger,
    ring: Finger,
    oh_cool: f64,
    wrist: i8,
}

impl HandState {
    fn new(wrist: i8) -> Self {
        Self {
            thumb: Finger::new(),
            index: Finger::new(),
            middle: Finger::new(),
            ring: Finger::new(),
            oh_cool: -1.0,
            wrist,
        }
    }

    fn max_finger_time(&self) -> f64 {
        self.thumb.last_move_time.max(
            self.index
                .last_move_time
                .max(self.middle.last_move_time.max(self.ring.last_move_time)),
        )
    }
}

#[derive(Debug)]
pub struct AlgSpeedConfig {
    ignore_errors: bool,
    ignore_auf: bool,
    wrist_mult: f64,
    push_mult: f64,
    ring_mult: f64,
    destabilize: f64,
    add_regrip: f64,
    double: f64,
    seslice_mult: f64,
    over_work_mult: f64,
    move_block: f64,
    rotation: f64,
}

impl Default for AlgSpeedConfig {
    fn default() -> Self {
        Self {
            ignore_errors: false,
            ignore_auf: false,
            wrist_mult: 0.8,
            push_mult: 1.3,
            ring_mult: 1.4,
            destabilize: 0.5,
            add_regrip: 1.0,
            double: 1.65,
            seslice_mult: 1.25,
            over_work_mult: 2.25,
            move_block: 0.8,
            rotation: 3.5,
        }
    }
}

pub struct AlgSpeed {
    config: AlgSpeedConfig,
}

impl AlgSpeed {
    pub fn new(config: AlgSpeedConfig) -> Self {
        Self { config }
    }

    fn calc_overwork(
        &self,
        finger: &Finger,
        location_prefer: Location,
        penalty: f64,
        speed: f64,
    ) -> f64 {
        if finger.location != location_prefer && speed - finger.last_move_time < penalty {
            penalty - speed + finger.last_move_time
        } else {
            0.0
        }
    }

    fn process_sequence(&self, sequence: &str) -> Result<f64, String> {
        let split_seq: Vec<&str> = sequence.split_whitespace().collect();
        let true_split_seq: Vec<String> = if self.config.ignore_errors {
            split_seq
                .into_iter()
                .filter(|&move_str| {
                    let valid_moves = [
                        "r", "r2", "r'", "u", "u'", "u2", "f", "f2", "f'", "d", "d2", "d'", "l",
                        "l2", "l'", "b", "b2", "b'", "m", "m2", "m'", "s", "s2", "s'", "e", "e2",
                        "e'", "x", "x'", "x2", "y", "y'", "y2", "z", "z'", "z2",
                    ];
                    valid_moves.contains(&move_str.to_lowercase().as_str())
                })
                .map(String::from)
                .collect()
        } else {
            split_seq.into_iter().map(String::from).collect()
        };

        let mut final_seq = true_split_seq;

        if self.config.ignore_auf {
            // Handle AUF at start
            if !final_seq.is_empty() {
                if final_seq[0].starts_with('U') {
                    final_seq.remove(0);
                } else if final_seq.len() >= 2
                    && final_seq[0].to_lowercase().starts_with('d')
                    && final_seq[1].starts_with('U')
                {
                    final_seq[1] = final_seq[0].clone();
                    final_seq.remove(0);
                }
            }

            // Handle AUF at end
            if !final_seq.is_empty() {
                let last_idx = final_seq.len() - 1;
                if final_seq[last_idx].starts_with('U') {
                    final_seq.pop();
                } else if final_seq.len() >= 2 {
                    let second_last_idx = final_seq.len() - 2;
                    if final_seq[last_idx].to_lowercase().starts_with('d')
                        && final_seq[second_last_idx].starts_with('U')
                    {
                        final_seq[second_last_idx] = final_seq[last_idx].clone();
                        final_seq.pop();
                    }
                }
            }
        }

        let initial_tests = vec![
            self.test_sequence(&final_seq, 0, 0, 0.0),
            self.test_sequence(&final_seq, 0, -1, 1.0 + self.config.add_regrip),
            self.test_sequence(&final_seq, 0, 1, 1.0 + self.config.add_regrip),
            self.test_sequence(&final_seq, -1, 0, 1.0 + self.config.add_regrip),
            self.test_sequence(&final_seq, 1, 0, 1.0 + self.config.add_regrip),
        ];

        self.find_best_speed(initial_tests, &final_seq)
    }

    fn test_sequence(
        &self,
        sequence: &[String],
        l_grip: i8,
        r_grip: i8,
        initial_speed: f64,
    ) -> TestResult {
        let mut left = HandState::new(l_grip);
        let mut right = HandState::new(r_grip);
        let mut speed = initial_speed;
        let mut grip = 1;
        let mut ud_grip = -1;
        // let mut prev_speed = None;
        // let mut first_move_speed = None;

        for (i, move_str) in sequence.iter().enumerate() {
            // Process move logic here...
            // This would be a very large match statement handling all possible moves
            // Similar to the JavaScript switch statement but in Rust style
        }

        TestResult {
            move_index: -1,
            speed,
            left_wrist: l_grip,
            right_wrist: r_grip,
            left_time: left.max_finger_time(),
            right_time: right.max_finger_time(),
        }
    }

    fn find_best_speed(
        &self,
        initial_tests: Vec<TestResult>,
        sequence: &[String],
    ) -> Result<f64, String> {
        // Implementation of the speed finding algorithm
        // This would replace the while(true) loop from JavaScript
        Ok(0.0) // Placeholder
    }
}

#[derive(Debug)]
struct TestResult {
    move_index: i32,
    speed: f64,
    left_wrist: i8,
    right_wrist: i8,
    left_time: f64,
    right_time: f64,
}
