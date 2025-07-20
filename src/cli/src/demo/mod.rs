use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use bevy::prelude::*;
use bevy_simple_text_input::TextInputPlugin;
use compiler::compile;
use cube_viz::CubeViz;
use internment::{ArcIntern, Intern};
use interpreter_loop::{CUBE3, CUBE3_DEF};
use interpreter_plugin::{CommandTx, InterpretationCommand, InterpreterPlugin};
use qter_core::{
    File, Int, Program,
    architectures::{Architecture, Permutation},
};

use crate::demo::{code_viz::CodeViz, io_viz::IOViz};

mod code_viz;
mod cube_viz;
mod interpreter_loop;
mod interpreter_plugin;
mod io_viz;

struct ProgramInfo {
    program: Arc<Program>,
    architecture: Arc<Architecture>,
    solved_goto_pieces: Vec<Vec<usize>>,
    code: String,
}

fn load_file(name: &str) -> Result<ArcIntern<str>, String> {
    let path = PathBuf::from(name);

    if path.ancestors().count() > 1 {
        // Easier not to implement relative paths and stuff
        return Err("Imported files must be in the same path".to_owned());
    }

    match fs::read_to_string(path) {
        Ok(s) => Ok(ArcIntern::from(s)),
        Err(e) => Err(e.to_string()),
    }
}

static PROGRAMS: LazyLock<HashMap<Intern<str>, ProgramInfo>> = LazyLock::new(|| {
    let mut programs = HashMap::new();

    programs.insert(
        Intern::from("simple"),
        ProgramInfo {
            program: Arc::new(
                compile(
                    &File::from(include_str!("../../../compiler/tests/simple/simple.qat")),
                    load_file,
                )
                .unwrap(),
            ),
            architecture: Arc::new(Architecture::new(Arc::clone(&CUBE3), &[vec!["U"]]).unwrap()),
            solved_goto_pieces: vec![
                vec![7, 18, 24], // UFR
            ],
            code: r#"
0 | input "First number:" U max-input 3
1 | input "Second number:" U max-input 3
2 | halt until UF solved U'
                "#
            .to_owned(),
        },
    );

    programs.insert(
        Intern::from("avg"),
        ProgramInfo {
            program: Arc::new(
                compile(
                    &File::from(include_str!(
                        "../../../compiler/tests/average/average_transform.qat"
                    )),
                    load_file,
                )
                .unwrap(),
            ),
            architecture: CUBE3_DEF.get_preset(&[90_u32, 90].map(Into::into)).unwrap(),
            solved_goto_pieces: vec![
                vec![23, 29, 42], // DFR
                vec![20, 27],     // FR
                vec![5, 16, 10],  // ULF
                vec![3, 9],       // UL
            ],
            code: r#"
0  | input "First number"
           R' F' L U' L U L F U' R
           max-input 90
1  | input "Second number"
           U F R' D' R2 F R' U' D
           max-input 90
2  | solved-goto DFR FR 5
3  | B2 R L2 D L' F' D2 F' L2
     B' U' R D' L' B2 R F
4  | goto 2
5  | R' F' L U' L U L F U' R
6  | R' U F' L' U' L' U L' F R
7  | solved-goto ULF UL 12
8  | R' U F' L' U' L' U L' F R
9  | solved-goto ULF UL 12
10 | U F R' D' R2 F R' U' D
11 | goto 7
12 | halt "The average is"
          D' U R F' R2 D R F' U'
          counting-until DFR FR
"#
            .to_owned(),
        },
    );

    programs.insert(
        Intern::from("fib"),
        ProgramInfo {
            program: Arc::new(
                compile(
                    &File::from(include_str!(
                        "../../../compiler/tests/fib/fib_transform.qat"
                    )),
                    load_file,
                )
                .unwrap(),
            ),
            architecture: CUBE3_DEF
                .get_preset(&[30_u32, 18, 10, 9].map(Into::into))
                .unwrap(),
            solved_goto_pieces: vec![
                vec![7, 18, 24],  // UFR
                vec![23, 29, 42], // DFR
                vec![6, 17],      // UF
                vec![20, 27],     // FR
                vec![14, 23],     // DL
                vec![5, 16, 10],  // UFL
            ],
            code: r#"
0  | input
     "Which Fibonacci number to calculate:"
     B2 U2 L F' R B L2 D2 B R' F L
     max-input 7
1  | solved-goto UFR 3
2  | goto 4
3  | halt "The number is: 0"
4  | D L' F L2 B L' F' L B' D' L'
5  | L' F' R B' D2 L2 B' R' F L' U2 B2
6  | solved-goto UFR 8
7  | goto 9
8  | halt "The number is"
          L D B L' F L B' L2 F' L D'
          counting-until DL DFL
9  | solved-goto DL DFL 12
10 | L U' B R' L B' L' U'
     L U R2 B R2 D2 R2 D'
11 | goto 9
12 | L' F' R B' D2 L2 B' R' F L' U2 B2
13 | solved-goto UFR 15
14 | goto 16
15 | halt "The number is"
          F2 L2 U2 D' R U' B L' B L' U'
          counting-until FR DRF
16 | solved-goto FR DRF 19
17 | D' B' U2 B D' F' D L' D2
     F' R' D2 F2 R F2 R2 U' R'
18 | goto 16
19 | L' F' R B' D2 L2 B' R' F L' U2 B2
20 | solved-goto UFR 22
21 | goto 23
22 | halt "The number is"
          U L' R' F' U' F' L' F2 L U R
          counting-until UF
23 | solved-goto UF 5
24 | B R2 D' R B D F2 U2 D'
     F' L2 F D2 F B2 D' L' U'
25 | goto 23
"#
            .to_owned(),
        },
    );

    programs.insert(
        Intern::from("multiply"),
        ProgramInfo {
            program: Arc::new(
                compile(
                    &File::from(include_str!(
                        "../../../compiler/tests/multiply/multiply_transform.qat"
                    )),
                    load_file,
                )
                .unwrap(),
            ),
            architecture: CUBE3_DEF
                .get_preset(&[30_u32, 30, 30].map(Into::into))
                .unwrap(),
            solved_goto_pieces: vec![
                vec![7, 18, 24],  // UFR
                vec![20, 27],     // FR
                vec![4, 25],      // UR
                vec![5, 16, 10],  // UFL
                vec![13, 39, 45], // DBL
                vec![1, 33],      // UB
            ],
            code: r#"
0   | input "Enter number X"
            L2 F2 U L' F D' F'
            U' L' F U D L' U'
            max-input 29
1   | input "Enter number Y"
            R2 L U' R' L2 F'
            D R' D L B2 D2
            max-input 29
2   | solved-goto FR UFR 145
3   | solved-goto UB DLB 148
4   | solved-goto UB 9
5   | F2 B2 U F2 B2 D' B L2
      D2 B' D B2 D L D2 B2 D'
6   | solved-goto UB 51
7   | F2 B2 U F2 B2 D' B L2
      D2 B' D B2 D L D2 B2 D'
8   | goto 4
9   | solved-goto ULF 12
10  | D B2 D2 L' D' B2 D' B D2
      L2 B' D F2 B2 U' F2 B2
11  | goto 9
12  | solved-goto UR ULF 15
13  | D' R L' U' F' B2 L B U B
      L U R' D2 B' U'
14  | goto 12
15  | solved-goto UB DLB 18
16  | F' B' D2 R' B2 R U R2 B2
      L' B' U B R2 L2 F R L'
17  | goto 15
18  | solved-goto ULF 21
19  | D B2 D2 L' D' B2 D' B D2
      L2 B' D F2 B2 U' F2 B2
20  | goto 18
21  | solved-goto UR ULF 24
22  | D' R L' U' F' B2 L B U B
      L U R' D2 B' U'
23  | goto 21
24  | solved-goto FR UFR 27
25  | B2 L F D R2 F R' U F' R2
      F D2 L2 D L B2
26  | goto 24
27  | solved-goto UR 30
28  | D2 L D' F' D' R D' R U2
      B R B2 U R' U F D
29  | goto 27
30  | solved-goto UR ULF 4
31  | F2 L' B' D F2 U' R2 F
      U2 R' D' B U2 F' L2 U
32  | goto 30
33  | solved-goto UB DLB 36
34  | U' R2 L2 U R2 F2 D2 R'
      F2 L' U2 L U L' B D' B
35  | goto 33
36  | solved-goto ULF 39
37  | D B2 D2 L' D' B2 D' B D2
      L2 B' D F2 B2 U' F2 B2
38  | goto 36
39  | solved-goto UR ULF 42
40  | D' R L' U' F' B2 L B
      U B L U R' D2 B' U'
41  | goto 39
42  | solved-goto FR UFR 45
43  | U' R' L2 B' L' D' F' R
      F' D R' L B2 R2 L2 U' R2
44  | goto 42
45  | solved-goto UR 48
46  | D2 L D' F' D' R D' R
      U2 B R B2 U R' U F D
47  | goto 45
48  | solved-goto UR ULF 57
49  | F2 L' B' D F2 U' R2 F
      U2 R' D' B U2 F' L2 U
50  | goto 48
51  | solved-goto ULF 54
52  | D B2 D2 L' D' B2 D' B D2
      L2 B' D F2 B2 U' F2 B2
53  | goto 51
54  | solved-goto UR ULF 57
55  | D' R L' U' F' B2 L B
      U B L U R' D2 B' U'
56  | goto 54
57  | solved-goto DLB 33
58  | D2 B2 L' D' R D'
      F R L2 U R2 L'
59  | solved-goto UB DLB 151
60  | goto 63
61  | solved-goto UB 73
62  | D2 B2 L' D' R D'
      F R L2 U R2 L'
63  | U L2 B' L U' B'
      U2 R B' R' B L
64  | solved-goto UB 97
65  | F2 B2 U F2 B2 D' B L2
      D2 B' D B2 D L D2 B2 D'
66  | solved-goto UB 97
67  | F2 B2 U F2 B2 D' B L2
      D2 B' D B2 D L D2 B2 D'
68  | solved-goto UB 97
69  | F2 B2 U F2 B2 D' B L2
      D2 B' D B2 D L D2 B2 D'
70  | solved-goto UB 97
71  | F2 B2 U F2 B2 D' B L2
      D2 B' D B2 D L D2 B2 D'
72  | goto 61
73  | solved-goto ULF 76
74  | D B2 D2 L' D' B2 D' B D2
      L2 B' D F2 B2 U' F2 B2
75  | goto 73
76  | solved-goto UR ULF 79
77  | D' R L' U' F' B2 L B
      U B L U R' D2 B' U'
78  | goto 76
79  | solved-goto UB DLB 82
80  | F2 B2 U F2 D L' B D2 B2
      D L2 D' B R' L2 B2 R L
81  | goto 79
82  | solved-goto ULF 85
83  | D B2 D2 L' D' B2 D' B D2
      L2 B' D F2 B2 U' F2 B2
84  | goto 82
85  | solved-goto UR ULF 88
86  | D' R L' U' F' B2 L B U
      B L U R' D2 B' U'
87  | goto 85
88  | solved-goto FR UFR 91
89  | D2 R' F' D2 R F R F' R'
      B' D F' L' D' B' L2 U' B2
90  | goto 88
91  | solved-goto UR 94
92  | D2 L D' F' D' R D' R
      U2 B R B2 U R' U F D
93  | goto 91
94  | solved-goto UR ULF 61
95  | F2 L' B' D F2 U' R2 F
      U2 R' D' B U2 F' L2 U
96  | goto 94
97  | solved-goto ULF 100
98  | D B2 D2 L' D' B2 D' B D2
      L2 B' D F2 B2 U' F2 B2
99  | goto 97
100 | solved-goto UR ULF 103
101 | D' R L' U' F' B2 L B
      U B L U R' D2 B' U'
102 | goto 100
103 | D2 B2 L' D' R D'
      F R L2 U R2 L'
104 | solved-goto UB 125
105 | R2 L U' R' L2 F'
      D R' D L B2 D2
106 | solved-goto UB DLB 109
107 | F' D' F' U' R B2 U2 D'
      R D F2 L B2 L D2 L2 D2
108 | goto 106
109 | solved-goto ULF 112
110 | D B2 D2 L' D' B2 D' B D2
      L2 B' D F2 B2 U' F2 B2
111 | goto 109
112 | solved-goto UR ULF 118
113 | D' R L' U' F' B2 L B
      U B L U R' D2 B' U'
114 | goto 112
115 | solved-goto FR UFR 118
116 | D2 F U2 R' U D2 F D'
      R D R2 D F' R U R
117 | goto 118
118 | solved-goto UR 121
119 | D2 L D' F' D' R D' R
      U2 B R B2 U R' U F D
120 | goto 118
121 | solved-goto UR ULF 103
122 | F2 L' B' D F2 U' R2 F
      U2 R' D' B U2 F' L2 U
123 | goto 121
124 | D2 B2 L' D' R
      D' F R L2 U R2 L'
125 | solved-goto UB DLB 151
126 | R2 L U' R' L2 F'
      D R' D L B2 D2
127 | solved-goto UB DLB 130
128 | R' B' R D F L2 U' B2 L2
      B' U L2 U L' U' B2 L2 F'
129 | goto 127
130 | solved-goto ULF 133
131 | D B2 D2 L' D' B2 D' B D2
      L2 B' D F2 B2 U' F2 B2
132 | goto 130
133 | solved-goto UR ULF 136
134 | D' R L' U' F' B2 L B
      U B L U R' D2 B' U'
135 | goto 133
136 | solved-goto FR UFR 139
137 | R' F2 D F' B2 L2 U L2 U
      F' B2 R D2 R' D' F2 D'
138 | goto 136
139 | solved-goto UR 142
140 | D2 L D' F' D' R D' R
      U2 B R B2 U R' U F D
141 | goto 139
142 | solved-goto UR ULF 124
143 | F2 L' B' D F2 U' R2 F
      U2 R' D' B U2 F' L2 U
144 | goto 142
145 | solved-goto UB DLB 151
146 | D2 B2 L' D' R
      D' F R L2 U R2 L'
147 | goto 145
148 | solved-goto FR UFR 151
149 | U L U' D' F' L U
      F D F' L U' F2 L2
150 | goto 148
151 | halt "(X * Y) mod 30 ="
         U L U' D' F' L U
         F D F' L U' F2 L2
         counting-until FR UFR
"#
            .to_owned(),
        },
    );

    programs
});

#[derive(Resource)]
struct CurrentState(Permutation);

pub fn demo(robot: bool) {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(InterpreterPlugin { robot })
        .add_plugins(CubeViz)
        .add_plugins(CodeViz)
        .add_plugins(IOViz)
        .add_plugins(TextInputPlugin)
        .add_systems(PreUpdate, keyboard_control)
        .run();
}

// Replace this with buttons
fn keyboard_control(keyboard_input: Res<ButtonInput<KeyCode>>, command_tx: Res<CommandTx>) {
    if keyboard_input.just_pressed(KeyCode::KeyN) {
        command_tx.send(InterpretationCommand::Step).unwrap();
    }

    if keyboard_input.just_pressed(KeyCode::KeyS) {
        command_tx.send(InterpretationCommand::Solve).unwrap();
    }

    if keyboard_input.just_pressed(KeyCode::KeyT) {
        command_tx
            .send(InterpretationCommand::Execute(Intern::from("multiply")))
            .unwrap();
    }
}
