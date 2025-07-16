use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use bevy::prelude::*;
use compiler::compile;
use cube_viz::CubeViz;
use internment::{ArcIntern, Intern};
use interpreter_loop::CUBE3_DEF;
use interpreter_plugin::{CommandTx, InterpretationCommand, InterpreterPlugin};
use qter_core::{
    File, Int, Program,
    architectures::{Architecture, Permutation},
};

mod cube_viz;
mod interpreter_loop;
mod interpreter_plugin;

struct ProgramInfo {
    program: Arc<Program>,
    architecture: Arc<Architecture>,
}

static PROGRAMS: LazyLock<HashMap<Intern<str>, ProgramInfo>> = LazyLock::new(|| {
    let mut programs = HashMap::new();

    programs.insert(
        Intern::from("test"),
        ProgramInfo {
            program: Arc::new(
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
            ),
            architecture: CUBE3_DEF
                .get_preset(&[Int::from(210_u32), Int::from(24_u32)])
                .unwrap(),
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
