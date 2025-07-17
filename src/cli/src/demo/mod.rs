use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use bevy::prelude::*;
use code_viz::CodeViz;
use compiler::compile;
use cube_viz::CubeViz;
use internment::{ArcIntern, Intern};
use interpreter_loop::CUBE3_DEF;
use interpreter_plugin::{CommandTx, InterpretationCommand, InterpreterPlugin};
use qter_core::{
    File, Int, Program,
    architectures::{Architecture, Permutation},
};

mod code_viz;
mod cube_viz;
mod interpreter_loop;
mod interpreter_plugin;

struct ProgramInfo {
    program: Arc<Program>,
    architecture: Arc<Architecture>,
    solved_goto_pieces: Vec<Vec<usize>>,
    code: String,
}

static PROGRAMS: LazyLock<HashMap<Intern<str>, ProgramInfo>> = LazyLock::new(|| {
    let mut programs = HashMap::new();

    let program = Arc::new(
        compile(&File::from(include_str!("../../test.qat")), |name| {
            let path = PathBuf::from(name);

            if path.ancestors().count() > 1 {
                // Easier not to implement relative paths and stuff
                return Err("Imported files must be in the same path".to_owned());
            }

            match fs::read_to_string(path) {
                Ok(s) => Ok(ArcIntern::from(s)),
                Err(e) => Err(e.to_string()),
            }
        })
        .unwrap(),
    );

    // println!("{:#?}", program.instructions);

    programs.insert(
        Intern::from("test"),
        ProgramInfo {
            program,
            architecture: CUBE3_DEF
                .get_preset(&[Int::from(210_u32), Int::from(24_u32)])
                .unwrap(),
            solved_goto_pieces: vec![
                vec![6, 17],      // UF
                vec![7, 18, 24],  // UFR
                vec![20, 27],     // FR
                vec![23, 29, 45], // FRD
            ],
            code: r#"0  | input "Number to modulus:"
           U R U' D2 B
           max-input 209
1  | U R' F U2 R' F L F2
     L' F U' F' U R2 U2
2  | solved-goto 1 UF UFR
3  | solved-goto 6 FRD FR
4  | F L2 F' B' U D R' U2
     R' U F U2 F D R U'
5  | goto 2
6  | solved-goto 9 UF UFR
7  | F L2 F' B' U D R' U2
     R' U F U2 F D R U'
8  | goto 6
9  | B' L U R' U' B2 L'
     D2 B2 D2 B2
10 | halt "The modulus is"
          B' D2 U R' U'
          counting-until FR FRD"#
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
            .send(InterpretationCommand::Execute(Intern::from("test")))
            .unwrap();
    }

    if keyboard_input.just_pressed(KeyCode::Enter) {
        command_tx
            .send(InterpretationCommand::GiveInput(Int::one()))
            .unwrap();
    }
}
