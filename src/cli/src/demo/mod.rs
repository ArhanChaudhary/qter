use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, LazyLock},
    thread,
};

use bevy::{
    DefaultPlugins,
    app::{App, Startup},
    core_pipeline::core_3d::Camera3d,
    ecs::{event::Event, resource::Resource, system::Commands},
    prelude::Deref,
};
use compiler::compile;
use crossbeam_channel::{Receiver, Sender, unbounded};
use internment::{ArcIntern, Intern};
use interpreter::puzzle_states::SimulatedPuzzle;
use qter_core::{Facelets, File, I, Int, Program, U, architectures::Permutation};

use crate::robot::{Cube3Robot, RobotLike};

mod interpreter_loop;

struct ProgramInfo {
    program: Arc<Program>,
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
        },
    );

    programs
});

#[derive(Event)]
enum InterpretationEvent {
    Message(String),
    Input(Int<U>),
    BeginHalt,
    HaltCountUp(Int<U>),
    CubeState(Permutation),
    SolvedGoto { facelets: Facelets },
    ExecutedInstruction { next_one: usize },
    BeganProgram(Intern<str>),
    FinishedProgram,
    // Stuff for highlighting instructions
}

#[derive(Resource, Deref)]
struct EventRx(Receiver<InterpretationEvent>);

enum InterpretationCommand {
    Execute(Intern<str>),
    Step,
    GiveInput(Int<I>),
    Solve,
}

#[derive(Resource, Deref)]
struct CommandTx(Sender<InterpretationCommand>);

fn setup<R: RobotLike + Send + 'static>(mut commands: Commands) {
    commands.spawn(Camera3d::default());

    let (event_tx, event_rx) = unbounded::<InterpretationEvent>();
    let (command_tx, command_rx) = unbounded::<InterpretationCommand>();

    thread::spawn(move || interpreter_loop::interpreter_loop::<R>(event_tx, command_rx));

    commands.insert_resource(EventRx(event_rx));
    commands.insert_resource(CommandTx(command_tx));
}

pub fn demo(robot: bool) {
    let mut app = App::new();
    let app = app
        .add_event::<InterpretationEvent>()
        .add_plugins(DefaultPlugins);

    if robot {
        app.add_systems(Startup, setup::<Cube3Robot>)
    } else {
        app.add_systems(Startup, setup::<SimulatedPuzzle>)
    }
    .run();
}
