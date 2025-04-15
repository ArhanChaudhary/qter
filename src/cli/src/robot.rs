use std::{
    cell::OnceCell,
    collections::HashMap,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Arc, OnceLock},
};

use interpreter::PuzzleState;
use itertools::Itertools;
use qter_core::architectures::{Algorithm, Permutation, PermutationGroup};

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

pub struct Cube3RobotPermutation {
    permutation: OnceCell<Permutation>,
    // robot_stdin: ChildStdin,
    // robot_stdout: ChildStdout,
    robot_process: Child,
}

impl PuzzleState for Cube3RobotPermutation {
    fn compose_into(&mut self, alg: &Algorithm) {
        println!("moves {}", alg.move_seq_iter().format(" "));
        self.permutation = OnceCell::new();
    }

    fn puzzle_state(&self) -> &Permutation {
        let input = io::stdin();
        let mut buffer = String::new();
        input.read_line(&mut buffer).unwrap();
        let rob_string = buffer.trim();
        self.puzzle_state_with_rob_string(rob_string)
    }

    fn identity(_perm_group: Arc<PermutationGroup>) -> Self {
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

        const ROBOT_EXECUTABLE_NAME: &str = "computeSystem";

        println!("Please enter the path to the robot executable:");

        let mut robot_path = String::new();
        io::stdin().read_line(&mut robot_path).unwrap();
        let robot_path = PathBuf::from(robot_path.trim()).join(ROBOT_EXECUTABLE_NAME);

        let mut robot_process = Command::new(robot_path)
            .arg("-noCameras")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let mut robot_stdin = robot_process.stdin.take().unwrap();
        let mut robot_stdout = robot_process.stdout.take().unwrap();
        println!("1");
        robot_stdin
            .write_all(b"p")
            .unwrap();
        robot_stdin.flush().unwrap();

        println!("2");
        let mut buffer = String::new();
        robot_stdout
            // .bytes()
            .take(1300)
            .read_to_string(&mut buffer)
            .unwrap();
        println!("{}", buffer);
        robot_print_string(&buffer);
        println!("4");

        // search for the string "Preset 7: Safe for Qter" inside the buffer
        if !buffer.contains("Preset 7: Safe for Qter") {
            panic!("Robot executable not found or not compatible");
        }

        Cube3RobotPermutation {
            permutation: OnceCell::new(),
            // robot_stdin,
            // robot_stdout,
            robot_process,
        }
    }
}

fn robot_print_string(s: &str) {
    let mut output = io::BufWriter::new(io::stdout());
    for line in s.lines() {
        output
            .write_all(format!("robot: {}\n", line).as_bytes())
            .unwrap();
    }
}

impl Cube3RobotPermutation {
    fn puzzle_state_with_rob_string(&self, rob_string: &str) -> &Permutation {
        assert_eq!(rob_string.len(), 54);

        let mut mapping: [usize; 48] = [0; 48];
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
        self.permutation
            .get_or_init(|| Permutation::from_mapping(mapping.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use internment::ArcIntern;
    use qter_core::architectures::PuzzleDefinition;

    // test caching

    #[test]
    fn thing2() {
        let perm_group = PuzzleDefinition::parse(include_str!("../../qter_core/puzzles/3x3.txt"))
            .unwrap()
            .perm_group;

        let solved = Permutation::identity(Arc::clone(&perm_group));
        let mut actual = Cube3RobotPermutation::identity(Arc::clone(&perm_group));

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
            expected.compose_into(&alg);
            actual.compose_into(&alg);

            assert_eq!(
                expected.puzzle_state().mapping(),
                actual.puzzle_state_with_rob_string(rob_string).mapping()
            );
        }
    }
}
