use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, LazyLock},
    thread,
};

use bevy::{
    DefaultPlugins,
    app::{App, Startup, Update},
    core_pipeline::core_3d::Camera3d,
    ecs::{
        event::{Event, EventReader, EventWriter},
        resource::Resource,
        system::{Commands, Res},
    },
    input::{ButtonInput, keyboard::KeyCode},
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

#[derive(Event, Debug)]
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

#[derive(Debug)]
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
        .add_plugins(DefaultPlugins)
        .add_systems(Update, keyboard_control)
        .add_systems(Update, read_events)
        .add_systems(Update, event_printer);

    if robot {
        app.add_systems(Startup, setup::<Cube3Robot>)
    } else {
        app.add_systems(Startup, setup::<SimulatedPuzzle>)
    }
    .run();
}

fn read_events(recv: Res<EventRx>, mut events: EventWriter<InterpretationEvent>) {
    for event in recv.try_iter() {
        events.write(event);
    }
}

// Replace this with UI
fn event_printer(mut reader: EventReader<InterpretationEvent>) {
    for event in reader.read() {
        println!("{event:?}");
    }
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
