use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
    fs::{self, File},
    io::{self, BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{ChildStdin, ChildStdout, Command, Stdio},
    sync::{Arc, OnceLock},
};

use interpreter::puzzle_states::{PuzzleState, SimulatedPuzzle};
use itertools::Itertools;
use qter_core::{
    Int, U,
    architectures::{Algorithm, Permutation, PermutationGroup},
    discrete_math::lcm_iter,
};

const ROB_CORNLETS: [[usize; 3]; 8] = [
    [8, 9, 20],
    [6, 18, 38],
    [0, 36, 47],
    [2, 45, 11],
    [29, 26, 15],
    [27, 44, 24],
    [33, 53, 42],
    [35, 17, 51],
];

const QTER_CORNLETS: [[usize; 3]; 8] = [
    [7, 24, 18],
    [5, 16, 10],
    [0, 8, 34],
    [2, 32, 26],
    [42, 23, 29],
    [40, 15, 21],
    [45, 39, 13],
    [47, 31, 37],
];

const ROB_EDGELETS: [[usize; 2]; 12] = [
    [5, 10],
    [7, 19],
    [3, 37],
    [1, 46],
    [32, 16],
    [28, 25],
    [30, 43],
    [34, 52],
    [23, 12],
    [21, 41],
    [50, 39],
    [48, 14],
];

const QTER_EDGELETS: [[usize; 2]; 12] = [
    [4, 25],
    [6, 17],
    [3, 9],
    [1, 33],
    [44, 30],
    [41, 22],
    [43, 14],
    [46, 38],
    [20, 27],
    [19, 12],
    [36, 11],
    [35, 28],
];

static CORNER_MAPPING: OnceLock<HashMap<[char; 3], (usize, [usize; 3])>> = OnceLock::new();
static EDGE_MAPPING: OnceLock<HashMap<[char; 2], (usize, [usize; 2])>> = OnceLock::new();

pub struct Cube3Robot {
    permutation: OnceCell<Permutation>,
    robot_stdin: RefCell<ChildStdin>,
    robot_stdout: RefCell<ChildStdout>,
    robot_path_buf: PathBuf,
    perm_group: Arc<PermutationGroup>,
    expected_perm: Permutation,
}

pub trait RobotLike {
    fn initialize(perm_group: Arc<PermutationGroup>) -> Self;

    fn compose_into(&mut self, alg: &Algorithm);

    fn take_picture(&self) -> &Permutation;

    fn solve(&mut self);
}

pub trait RobotLikeDyn {
    fn compose_into(&mut self, alg: &Algorithm);

    fn take_picture(&self) -> &Permutation;

    fn solve(&mut self);
}

impl<R: RobotLike> RobotLikeDyn for R {
    fn compose_into(&mut self, alg: &Algorithm) {
        <Self as RobotLike>::compose_into(self, alg);
    }

    fn take_picture(&self) -> &Permutation {
        <Self as RobotLike>::take_picture(self)
    }

    fn solve(&mut self) {
        <Self as RobotLike>::solve(self);
    }
}

impl RobotLike for Cube3Robot {
    fn initialize(perm_group: Arc<PermutationGroup>) -> Self {
        init_mapping();

        println!("Robot debugging? (y/n)");
        let mut debug = String::new();
        io::stdin().read_line(&mut debug).unwrap();
        let debug = debug.trim() == "y";

        let mut robot_path = String::new();
        let robot_path = if debug {
            "/Users/arhan/Desktop/Compute_System-main"
        } else {
            println!("Please enter the path to the robot source:");
            io::stdin().read_line(&mut robot_path).unwrap();
            robot_path.trim()
        };
        let robot_path_buf = PathBuf::from(robot_path);
        let robot_executable_path = robot_path_buf.join("computeSystem");

        let mut robot_command = Command::new(robot_executable_path);
        robot_command
            .current_dir(robot_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());

        if debug {
            robot_command.arg("-noCameras").arg("-debug");
        }

        #[allow(clippy::zombie_processes)]
        let mut robot_process = robot_command.spawn().unwrap();

        let robot_stdin = RefCell::new(robot_process.stdin.take().unwrap());
        let robot_stdout = RefCell::new(robot_process.stdout.take().unwrap());
        let mut ret = Cube3Robot {
            permutation: OnceCell::new(),
            robot_stdin,
            robot_stdout,
            robot_path_buf,
            expected_perm: perm_group.identity(),
            perm_group,
        };

        ret.robot_tui(
            &["p", "7"],
            &["Preset 7: Safe for Qter", "[ Enter ] Ready to Solve"],
            "[  Esc  ] Exit Program",
        );
        RobotLike::solve(&mut ret);
        ret
    }

    fn compose_into(&mut self, alg: &Algorithm) {
        self.permutation = OnceCell::new();

        self.expected_perm.compose_into(alg.permutation());

        let moves_file_path = self.robot_path_buf.join("resource/testSequences/tmp.txt");
        let mut moves_file = File::create(moves_file_path).unwrap();
        let chunk = alg.move_seq_iter().format(" ").to_string();
        moves_file.write_all(chunk.as_bytes()).unwrap();

        self.robot_tui(
            &["t", "1\n", "0\n"],
            &["1. tmp.txt", "1. tmp.txt", "[  Esc  ] Exit Program"],
            "[  Esc  ] Exit Program",
        );
    }

    fn take_picture(&self) -> &Permutation {
        self.puzzle_state()
    }

    fn solve(&mut self) {
        self.robot_tui(
            &["\n", "\n"],
            &["[ Enter ] Start the Solve", "Total Time: "],
            "[   C   ] Print Cube State",
        );
    }
}

impl RobotLike for SimulatedPuzzle {
    fn initialize(perm_group: Arc<PermutationGroup>) -> Self {
        <Self as PuzzleState>::initialize(perm_group)
    }

    fn compose_into(&mut self, alg: &Algorithm) {
        // thread::sleep(Duration::from_millis(500));
        <Self as PuzzleState>::compose_into(self, alg);
    }

    fn take_picture(&self) -> &Permutation {
        // thread::sleep(Duration::from_millis(500));
        self.puzzle_state()
    }

    fn solve(&mut self) {
        <Self as PuzzleState>::solve(self);
    }
}

pub struct RobotState<R: RobotLike> {
    robot: R,
    perm_group: Arc<PermutationGroup>,
}

impl<R: RobotLike> PuzzleState for RobotState<R> {
    fn compose_into(&mut self, alg: &Algorithm) {
        self.robot.compose_into(alg);
    }

    fn initialize(perm_group: Arc<PermutationGroup>) -> Self {
        RobotState {
            perm_group: Arc::clone(&perm_group),
            robot: R::initialize(perm_group),
        }
    }

    fn facelets_solved(&self, facelets: &[usize]) -> bool {
        let state = self.robot.take_picture();

        for &facelet in facelets {
            let maps_to = state.mapping()[facelet];
            if self.perm_group.facelet_colors()[maps_to]
                != self.perm_group.facelet_colors()[facelet]
            {
                return false;
            }
        }

        true
    }

    fn print(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>> {
        let before = self.robot.take_picture().to_owned();

        let c = self.halt(facelets, generator)?;

        let mut exponentiated = generator.to_owned();
        exponentiated.exponentiate(c.into());

        self.compose_into(&exponentiated);

        if &before != self.robot.take_picture() {
            eprintln!("Printing did not return the cube to the original state!");
            return None;
        }
        Some(c)
    }

    fn halt(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>> {
        let mut generator = generator.to_owned();
        generator.exponentiate(-Int::<U>::one());

        let mut sum = Int::<U>::zero();

        let chromatic_orders = generator.chromatic_orders_by_facelets();
        let order = lcm_iter(facelets.iter().map(|&i| chromatic_orders[i]));

        while !self.facelets_solved(facelets) {
            sum += Int::<U>::one();

            if sum >= order {
                eprintln!(
                    "Decoding failure! Performed as many cycles as the size of the register."
                );
                return None;
            }

            self.compose_into(&generator);
        }

        Some(sum)
    }

    fn repeat_until(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<()> {
        // Halting has the same behavior as repeat_until
        self.halt(facelets, generator).map(|_| ())
    }

    fn solve(&mut self) {
        todo!()
    }
}

fn init_mapping() {
    CORNER_MAPPING.get_or_init(|| {
        let mut corner_mapping = HashMap::new();
        for (i, block) in [
            ['U', 'R', 'F'],
            ['U', 'F', 'L'],
            ['U', 'L', 'B'],
            ['U', 'B', 'R'],
            ['D', 'F', 'R'],
            ['D', 'L', 'F'],
            ['D', 'B', 'L'],
            ['D', 'R', 'B'],
        ]
        .into_iter()
        .enumerate()
        {
            for perm in (0..3).permutations(3) {
                let perm: [usize; 3] = perm.try_into().unwrap();
                let block = [block[perm[0]], block[perm[1]], block[perm[2]]];
                corner_mapping.insert(block, (i, perm));
            }
        }
        corner_mapping
    });
    EDGE_MAPPING.get_or_init(|| {
        let mut edge_mapping = HashMap::new();
        for (i, block) in [
            ['U', 'R'],
            ['U', 'F'],
            ['U', 'L'],
            ['U', 'B'],
            ['D', 'R'],
            ['D', 'F'],
            ['D', 'L'],
            ['D', 'B'],
            ['F', 'R'],
            ['F', 'L'],
            ['B', 'L'],
            ['B', 'R'],
        ]
        .into_iter()
        .enumerate()
        {
            for perm in (0..2).permutations(2) {
                let perm: [usize; 2] = perm.try_into().unwrap();
                let block = [block[perm[0]], block[perm[1]]];
                edge_mapping.insert(block, (i, perm));
            }
        }
        edge_mapping
    });
}

fn robot_debug(s: &str) {
    eprintln!("robot: {s:?}");
}

fn qter_debug(s: &str) {
    eprintln!("qter: sending {s:?} to robot");
}

impl Drop for Cube3Robot {
    fn drop(&mut self) {
        let moves_file_path = self.robot_path_buf.join("resource/testSequences/tmp.txt");
        let _ = fs::remove_file(moves_file_path);
    }
}

impl Cube3Robot {
    fn robot_tui(&self, ins: &[&str], expecteds: &[&str], ending: &str) {
        assert_eq!(ins.len(), expecteds.len());

        let mut robot_stdin = self.robot_stdin.borrow_mut();
        let mut robot_stdout = self.robot_stdout.borrow_mut();
        let mut robot_stdout = BufReader::new(&mut *robot_stdout);

        for (i, (in_, expected)) in ins.iter().zip(expecteds.iter()).enumerate() {
            qter_debug(in_);
            robot_stdin.write_all(in_.as_bytes()).unwrap();
            robot_stdin.flush().unwrap();

            let mut stdout_valid = false;
            for line in robot_stdout.by_ref().lines() {
                let line = line.unwrap();
                robot_debug(&line);
                if line.contains(expected) {
                    stdout_valid = true;
                    if i != ins.len() - 1 {
                        break;
                    }
                }
                if i == ins.len() - 1 && line.contains(ending) {
                    break;
                }
            }

            assert!(
                stdout_valid,
                "Expected {expected:?} as output from robot executable"
            );
        }
    }

    fn puzzle_state(&self) -> &Permutation {
        let perm = self.permutation.get_or_init(|| {
            let mut robot_stdin = self.robot_stdin.borrow_mut();
            let mut robot_stdout = self.robot_stdout.borrow_mut();
            let mut robot_stdout = BufReader::new(&mut *robot_stdout);

            let in_ = "c";
            let expected1 = "Current Cube State String:";
            let expected2 = "Is legal cube state?:";
            let ending = "[  Esc  ] Exit Program";
            let mut ret = None;

            while ret.is_none() {
                qter_debug(in_);
                robot_stdin.write_all(in_.as_bytes()).unwrap();
                robot_stdin.flush().unwrap();

                let mut rob_string = None;
                for line in robot_stdout.by_ref().lines().map(|line| line.unwrap()) {
                    robot_debug(&line);
                    if rob_string.is_none() && ret.is_none() {
                        if line.contains(expected1) {
                            rob_string = Some(line[expected1.len()..].trim().to_string());
                            // let mut buffer = String::new();
                            // io::stdin().read_line(&mut buffer).unwrap();
                            // rob_string = Some(buffer.trim().to_string());
                        }
                    } else {
                        if ret.is_none() && line.contains(expected2) {
                            match line[expected2.len()..].trim() {
                                "Yes" => {
                                    ret = Some(Self::puzzle_state_with_rob_string(
                                        rob_string.as_ref().unwrap(),
                                    ));
                                }
                                "No" => (),
                                _ => {
                                    panic!(
                                        "Expected 'Yes' or 'No' as output from robot at {line:?}"
                                    );
                                }
                            }
                        }
                        if line.contains(ending) {
                            break;
                        }
                    }
                }
                if ret.is_none() {
                    eprintln!("qter: Invalid cube state, retrying photo...");
                }
            }

            ret.unwrap()
        });

        if perm != &self.expected_perm {
            eprintln!(
                "WARNING: The cube state found by the robot is different than what is expected."
            );
            eprintln!("Expected: {:?}", self.expected_perm);
            eprintln!("Found: {perm:?}");
        }

        perm
    }

    fn puzzle_state_with_rob_string(rob_string: &str) -> Permutation {
        assert_eq!(rob_string.len(), 54);

        let mut mapping = vec![0; 48];
        for (i, corner) in ROB_CORNLETS.iter().enumerate() {
            let mut block: [char; 3] = Default::default();
            for j in 0..3 {
                block[j] = rob_string.chars().nth(corner[j]).unwrap();
            }
            let (hash, mapping_order) = CORNER_MAPPING
                .get_or_init(|| unreachable!())
                .get(&block)
                .copied()
                .unwrap();
            for j in 0..3 {
                mapping[QTER_CORNLETS[hash][mapping_order[j]]] = QTER_CORNLETS[i][j];
            }
        }

        for (i, edge) in ROB_EDGELETS.iter().enumerate() {
            let mut block: [char; 2] = Default::default();
            for j in 0..2 {
                block[j] = rob_string.chars().nth(edge[j]).unwrap();
            }
            let (hash, mapping_order) = EDGE_MAPPING
                .get_or_init(|| unreachable!())
                .get(&block)
                .copied()
                .unwrap();
            for j in 0..2 {
                mapping[QTER_EDGELETS[hash][mapping_order[j]]] = QTER_EDGELETS[i][j];
            }
        }
        Permutation::from_mapping(mapping)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::Parser;
    use internment::ArcIntern;
    use interpreter::puzzle_states::SimulatedPuzzle;
    use qter_core::architectures::puzzle_definition;

    #[test]
    fn test_puzzle_state_with_rob_string() {
        init_mapping();
        let perm_group = Arc::clone(
            &puzzle_definition()
                .parse(qter_core::File::from("3x3"))
                .unwrap()
                .perm_group,
        );

        let solved = <SimulatedPuzzle as RobotLike>::initialize(Arc::clone(&perm_group));

        for [seq, rob_string] in [
            ["", "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB"],
            [
                "U",
                "UUUUUUUUUBBBRRRRRRRRRFFFFFFDDDDDDDDDFFFLLLLLLLLLBBBBBB",
            ],
            [
                "U2 R2 L D2 L F2 B2 U' D' F U R' L2 U2 D L F' B2 D R2",
                "FLLLULFRFRUURRRBBBLDDFFUBRUDFRDDUFLDUFUFLDLBRBDRBBULBD",
            ],
            [
                "L U' R2 F B2 R2 L U' D2 R2 F B' U R' U2 R2 D2 F2 U2 L'",
                "BLBLUFFFFDULURFRRULDLBFRDBUFDFBDDDBRDUUFLULRRUDRLBRBLB",
            ],
            [
                "R L2 U2 D2 R2 U2 R' D2 R' F' R L2 B R2 L' F' B2 U' D' F B2 U R' L2",
                "DUULUFBDDRRFURDBFLRBBUFBLBLULURDLLBDFUUFLRBDFRFRLBDFRD",
            ],
            [
                "L2 U2 D' L2 U2 L2 F2 U2 D' F2 B2 L2 U B U2 D R2 U' F2 D R2 L'",
                "BULRUDFDDFLDRRRBFBLFRUFFFBLULUFDLLDRRDURLLBURFBUBBUDBD",
            ],
            [
                "U2 L D2 R2 U2 L F2 B2 D' R' L' F' B2 R' L2 F2 B' R U2 D F B2 R L'",
                "BRRRUBLRBDDFBRRUDRBDRLFLDLFLURDDFUFBLBUBLFLLFDUDFBUUUF",
            ],
            [
                "F U2 D2 R2 F' B2 D2 F B D2 L2 U2 D R F2 B' L' B R L U D2 R2 L'",
                "RFDUUDBFRFBFRRDBBUDRUDFBBDLLLULDUDLLUFRRLRRUDLLBFBUFBF",
            ],
            [
                "U D F' B' L' B R L2 F B R2 L' U F2 B R2 F2 B2 U' F2 B2 U D R2 D'",
                "BDUFULLBFRBRURBFUBFRDUFRUFRFRULDBDLRUDDFLFLULBRLDBLDDB",
            ],
            [
                "R2 D' B2 U' D' R2 U' L2 U' L2 B' R' F D F' B U2 L'",
                "BUDFUDRFDBRFURBFDRDULRFBUBURDLFDLRDDULFULBBRFLRLLBLBFU",
            ],
            [
                "F2 L' D2 F2 B2 R' U2 B2 R' F' B2 R' D F R2 L2 D' R L2 F2 B R2",
                "LFDRUFUBLFRBURDBRRFLDFFLBFLRUUUDUULFDBLDLDFBURLBRBBDDR",
            ],
            [
                "F2 U F2 R2 L2 U2 D' L2 U2 D2 F2 B R2 L U D' L' U2 R' L2 F B2 R'",
                "FBLDUUBFUBFDLRUDLRLDRRFFUDRRLFRDULDDLBDBLUUFFFRUBBLBRB",
            ],
            [
                "R2 B2 D2 R2 U2 R' L2 F' B R2 L' D' F2 B' U' R' L2 F U2 D F B'",
                "BDRDURBLLFFDBRFRBLUUDUFLUDFRBDUDRLFFRRLDLRBBFBFUUBLULD",
            ],
            [
                "F2 U D2 R2 D' F' D R L2 U2 D B2 R' U2 D2 R L2 B2 R' L' U2 L'",
                "FFBRUDBDDRLDRRBLFFUFBBFDBBFRRURDUFURRULLLULFULLDDBBULD",
            ],
            [
                "F2 B2 R2 L' U2 R2 U2 D2 F2 R2 L' F2 L U B2 U2 D' L U2 D2 F' B' D' F2 B2",
                "RUFDUDBFLFLRBRDLUBLLDLFRBFFURUBDLRBRUFDULBFDLURBRBFDUD",
            ],
            [
                "L D2 R' L' D2 F' R' L' F B2 U' D F B' R' U2 L",
                "FRFDUDULRUBDFRBFFRLDBLFRDULRRDLDUBLULFBBLFDBBRDURBUFUL",
            ],
            [
                "U D' B2 U D2 B R' L F B2 R2 L' U D F2 U2 D2 R L2 D2 R2 L B2 L D2 B2",
                "LFFLURDLULBLURURDRBUBDFRRFUDDFFDLBRBUBLULBRLFDRFFBBUDD",
            ],
            [
                "L2 F2 B2 U' L2 U D2 F2 B2 U2 L2 B' U D2 L U D F' U2 B' R2 L'",
                "RBBBURBLLDFRRRFULUUUBDFBRULUFFRDDLUFDLLRLFDUBDDFLBDRBF",
            ],
            [
                "U' R2 L2 F B R' F2 B2 D' F' R U R2 B2 R2 F2 U' R2 L2 U' D",
                "URFLUDULULFLDRRRLBRBBRFBRBDDUBDDULRDBFFFLFFLFDURBBULDU",
            ],
            [
                "L2 F U2 D' R L2 B L' U' D' F' R2 F' D2 F' B' U2 L2 F B2 D2",
                "LBLUULUFBLDUFRUDFRRLDDFRFUBDRRDDDDBBUBFFLBRRLBRFLBUULF",
            ],
            [
                "B' U2 B U2 D R' L' D F B R F' D2 L2 F' R2 L2 F' D2 F2 R2 L2",
                "UFLBURFFLBFBLRRFRRLUUDFUBBDDRRFDDFBBRUULLLLLRDDFUBBUDD",
            ],
            [
                "U2 D2 F2 D2 F2 B2 R2 L' F2 R' L2 B' L' U2 D' B2 R' L' U D2 R L U D2 F B2",
                "RDDBUFRBUBLRURRLFLFLRLFRFBBDUDLDDUFFBRUULDBULFBDDBFURL",
            ],
            [
                "R2 D2 B' D2 F B L2 U2 D2 F2 L' U D F' B R F R' L2 B",
                "RUURULDDRBFLFRRBLLLLDUFRRUUDBRDDBLRBFBFDLFUFFFLUDBBDUB",
            ],
            [
                "D L' F B' R L2 D' R2 F' B R L2 F L2 D2 F2 R2 L2 F' B2 U2 F2 B",
                "DRDRULBDUBFFDRFLBDURRUFLUFFRUUBDUBBBFFLBLLDLFLURDBRRDL",
            ],
            [
                "R L2 U2 R2 L B2 L' F2 U2 R' U2 D' F' R' L2 F2 B R L' F2 R2 L' D' F2",
                "DFBLURFFDBDLDRLUFFLLLDFBDFRFRBRDDDURRBURLLFURUUBUBBUBL",
            ],
            [
                "F B2 R2 L F B R2 L U' F2 B R2 L' F B R2 L2 U' D2 R2 U D2 B2 R2 L2 U R2 L2",
                "URDRULRDLBURLRBFUDURUFFBDDUBLLBDFRRFRUBDLLDULFFFDBFLBB",
            ],
            [
                "R L2 U2 D2 F2 L U' D F B' U L B' L2 B' U2 F2 B R2 L2 D2",
                "LFRLUULLLBBDURRDBBDBUBFFULRFDFRDRFFUFUBFLDRDLBDDUBLRRU",
            ],
            [
                "L2 B2 R2 U' D2 B2 R2 L2 U' D' B2 U' R2 L U F L F2 B' U D2 R' L'",
                "RUBRULRDLFFUFRULFUDLDUFRLBFBRULDDDBFBDFBLRLUURBDFBLRDB",
            ],
            [
                "F U' F' B' L' U F B R F' B D2 F B2 U2 R2 U2 D2 F B",
                "RRDBUFDLBLLFLRRURBRUDLFDLRRUUBUDBRBLDDBDLBUFFLDFFBFUUF",
            ],
            [
                "F U2 F' B' R U2 L U' D' B' R F2 B2 R U2 F2 B2 L F2 U2 D2 R",
                "FLDUUFRLULRLURRUBDDDBRFRBLLDFFUDLRBFUFFFLBBBRBURDBDLDU",
            ],
            [
                "R' L2 B' U D F2 B' R' U2 B' R' L F2 B D2 F2 B' D2 L2 F2 B' R2 U2 D2",
                "UFLLUUBDBRRBFRDFUDDLUUFRFBUULLLDBDDFLFRBLFRURDDBBBRLRF",
            ],
            [
                "L' U2 D2 F2 R' L2 U2 R' F2 B2 R' F2 B2 D F2 B U2 L' U2 D' R' L U' D' F2 B2",
                "RDLLUUDBFLRURRFDDLRLUDFBURLBFBUDBFLDFDBFLRDBRBFUUBLFUR",
            ],
            [
                "F2 R' L' B' U' D R L2 F2 B' R L U2 D' F2 B' R2 F R2 L2 U2 B' U2 R2",
                "BUFFUFULBLDDURDLFRRDDFFBDLULBBBDLFRBDRFRLUUDFRLRRBUUBL",
            ],
            [
                "R L2 U2 R2 L F2 B R' U' D2 R' L U F B L2 U2 L D2 F2 R' F2",
                "RDFUUBFLBLUUDRFUDDRBDUFLUFFLRRBDBBFFBLDLLRURBLRDUBFLDR",
            ],
            [
                "F2 B R' F' R L' U D2 F B2 L U R' D2 R2 F' B2 L2 U2 F' B'",
                "BRDBUURFFDFRURUDLRFDLFFRURBFFRLDUBRBLLUBLLLDLFBDBBDUDU",
            ],
            [
                "U2 F2 B2 R L F2 R L2 U2 R L U D' F' B U F' R L2 F2 R2 L U' D' R L'",
                "FLBDURDBRUFLBRLLDFLUBFFRDRBFUDLDRBURLFFBLLRURUDUBBDUFD",
            ],
            [
                "F B2 R L2 U2 D R L2 D2 R2 L2 F' B' L F2 B' L F2 B2 D2 R L2 B2 R' L2 F2 B2",
                "RLURUURBDFRBURDLBRBURLFFLDDDLFDDLFRBFFDDLUURBLFUFBBUBL",
            ],
            [
                "B2 U2 D2 R L2 D2 R2 L' U2 D2 L B2 R D F2 B2 R2 L' U2 D R2 L2 F' B2 D' R' L2",
                "LUDRUDLRURBRFRLBBDUUFDFUBLLLFUDDRRUBFDFFLLBFDFLDBBRRBU",
            ],
            [
                "R' L2 U2 L2 U2 B2 R L2 U' D2 R L2 F2 B2 U D F' B2 U D' F2 R U D",
                "DLRBUUBRLUBBLRLBFRLFFBFBUDRLLDUDDDUUFRDRLDFRBUFRUBDFFL",
            ],
            [
                "U D2 B' R L' B R2 B' R' U' F' B U2 D2 L2 F2 U2 R2 L",
                "UFDLUFDUURDFLRUBRLFLFLFDDFLRRUBDBBRBFFRRLBUDBLULBBUDDR",
            ],
            [
                "L2 F2 B2 U D2 R2 D R2 L2 F2 R L2 F' B' U D2 R L2 U2 F' B' R' L2 U2 D R' L",
                "LFLFULLDRUDUURBDLFFRBRFLLUBDRRFDFFURUDDLLBDRBBUFDBBUBR",
            ],
            [
                "F B R L' F L U' D' B' U D F B2 D B2 D2 F2 R B2 D2",
                "URFUUDRLLDRRBRFDFLDBBDFRLDBUBRUDLBDDBLFBLLLFFUURRBUFFU",
            ],
            [
                "R' B2 L B2 R' F B U' D2 F' B2 U' D R F' D F B' U D",
                "FRURUDRBFDBBFRLRRDBLLLFDDLBFFUFDUFDLUFDBLDUURLBRUBUBRL",
            ],
            [
                "B R' L' U' F2 B' R' U D2 F2 B' D2 R2 L2 F' R2 F B2 U2 R2 D2 R2 L2",
                "FRLBUDDDFDFURRFLLUBBLFFBBBFDLULDDRRRUULFLRFURBDRUBLBUD",
            ],
            [
                "F2 R2 L' F2 U2 L B2 D2 R L2 U D R2 L' U' F' B U F D' B' R' L2",
                "RUUUURFFLDULBRUBDFDLBRFRBBLDDURDLRBRDLLFLDBFRFFFBBDULU",
            ],
            [
                "U2 D F' B' U' F' R' U D R U' R2 L F2 B' D2 B2 D2 R2 B'",
                "DRURUFBDDBUFFRDFFUDBLUFLFRURBLRDDLBRLDRBLLUUDRFFLBUBLB",
            ],
            [
                "B U' D2 F2 B' U2 D' L' U D2 F2 B R L' U F2 U2 D' F2 U' D' B2 R2 F2",
                "DURRURUFLDFDBRDRDFLUBUFDFRBUUUFDLDDULBBBLBRLRBLFFBLLRF",
            ],
            [
                "L' F B2 L D' R' L' U' F2 B U' R2 F' U2 R2 L2 F D2 B' L2 U2",
                "DUUDUBBDUFURLRFBURUBLRFDFBRDRDBDFBFUFRLRLFDLLBLRDBUFLL",
            ],
            [
                "F2 B2 R' L2 D2 R' U2 D2 F2 R2 L U' D2 L2 D L' F B2 U D2 F2 R L' F'",
                "LUURUFBRBDDRURLLLRLDRLFFDUUFBBBDDLLUUFDDLFDRRBRFBBBFUF",
            ],
            [
                "R F' B' R B2 U F B2 U D F2 B U' D2 R2 B2 D2 F2 D R2",
                "FRDRUUFLBLFBRRFDLRLBDUFDLLFBDLDDUFRBUFULLBDBURURDBFUBR",
            ],
            [
                "D2 R B2 L' D2 R U D2 B2 R' L' U2 D' F U2 R2 L F' R2 L'",
                "LLRLUUDRURLFURFRLDRBFRFBLDBFRUDDFLFLBDBFLUFBDDBUDBUBRU",
            ],
            [
                "U2 D2 R L2 F2 B L D R L B' U' R2 L U2 D F2 L2 U' D' R2 U' R2",
                "LDURUFUURBUBLRBBFUBRDDFFFLDRDLBDRRFLFBRLLRULDLBDUBUFDF",
            ],
            [
                "F2 B2 R' U2 R' F2 D2 R F2 B' R2 D' F U F2 B' R L2 U' D'",
                "LRDDUDBUUFFRDRBBUUDFLRFRLBDBLLFDLULFFBRULURLUFFDRBBRDB",
            ],
            [
                "F2 B R L F' B2 L2 U' D2 R2 L' F L' U D2 F' U2 D2 F B D2 R2 F B2",
                "FLLFULDLFUBBBRFBBULDRUFULFRBDDRDDUULRLFBLRRDUDUDRBRFFB",
            ],
            [
                "B L2 U2 D F B R2 U' D2 R' F' B2 U R2 B2 U' D' B2 D R2 U2 D' R2 L2",
                "BBLUUDLDUBBFURLDFBFLRLFFLRRDBFRDDFRRULDULBRFBUULFBRDDU",
            ],
            [
                "R2 U2 L' B2 R2 L' B2 U2 D2 L' F' B2 U' F' B' U2 D' F' R' U' L",
                "FLUBULFRDBUBFRUBFFUDLDFDDLURBRFDRRBLLURFLBFLBLDURBUDRD",
            ],
            [
                "U F2 B2 U B2 U2 D R' L U' D' F' B2 D F2 B U' R2 F' B2 U2 D'",
                "DDFBUFDRDFURFRFULBRURBFDULLBDFFDUBBDLDBBLULLRURFRBLLRU",
            ],
            [
                "U R' L2 U2 D R L U R B R' L F B U' D2 F2 R2 U' D2 L2 U' R2",
                "ULDBUDRLUFRRURBBDFBULUFFRDLFFURDLLLLBRDULRBFUFBRDBBDFD",
            ],
            [
                "U' F2 U2 F2 B2 U F2 R2 F U' D R' L' U' D2 F2 B D2 F' B2 R' L'",
                "BBRBULUFBUFULRULFLFDLBFURRDUFFLDUFBBDDLRLRDDBFURRBDDLR",
            ],
            [
                "U2 D2 L2 F' B L2 B2 U2 F' R L2 F' B2 U2 D F' U' D2 R U2 D F2",
                "LBDLULDDURDRRRFRURBFFRFBFRDLDBDDBDFBBULRLFFBUFLUUBUULL",
            ],
        ] {
            let mut expected = solved.clone();

            let alg = Algorithm::new_from_move_seq(
                Arc::clone(&perm_group),
                seq.split_ascii_whitespace()
                    .map(ArcIntern::from)
                    .collect_vec(),
            )
            .unwrap();
            <SimulatedPuzzle as RobotLike>::compose_into(&mut expected, &alg);

            assert_eq!(
                expected.puzzle_state().mapping(),
                Cube3Robot::puzzle_state_with_rob_string(rob_string).mapping()
            );
        }
    }
}
