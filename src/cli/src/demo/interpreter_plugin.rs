use std::thread;

use bevy::{
    app::{Plugin, Startup, Update},
    ecs::{
        event::{Event, EventWriter},
        resource::Resource,
        system::{Commands, Res},
    },
    prelude::Deref,
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use internment::Intern;
use interpreter::puzzle_states::SimulatedPuzzle;
use qter_core::{Facelets, I, Int, U, architectures::Permutation};

use crate::robot::{Cube3Robot, RobotLike};

use super::interpreter_loop;

pub struct InterpreterPlugin {
    pub robot: bool,
}

#[derive(Event)]
pub struct Message(pub String);

#[derive(Event)]
pub struct Input(pub Int<U>);

#[derive(Event)]
pub struct BeginHalt;

#[derive(Event)]
pub struct HaltCountUp(pub Int<U>);

#[derive(Event)]
pub struct CubeState(pub Permutation);

#[derive(Event)]
pub struct SolvedGoto {
    pub facelets: Facelets,
}

#[derive(Event)]
pub struct ExecutedInstruction {
    pub next_one: usize,
}

#[derive(Event)]
pub struct BeganProgram(Intern<str>);

#[derive(Event)]
pub struct FinishedProgram;

#[derive(Debug)]
pub enum InterpretationEvent {
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
pub enum InterpretationCommand {
    Execute(Intern<str>),
    Step,
    GiveInput(Int<I>),
    Solve,
}

#[derive(Resource, Deref)]
pub struct CommandTx(Sender<InterpretationCommand>);

fn setup<R: RobotLike + Send + 'static>(mut commands: Commands) {
    let (event_tx, event_rx) = unbounded::<InterpretationEvent>();
    let (command_tx, command_rx) = unbounded::<InterpretationCommand>();

    thread::spawn(move || interpreter_loop::interpreter_loop::<R>(event_tx, command_rx));

    commands.insert_resource(EventRx(event_rx));
    commands.insert_resource(CommandTx(command_tx));
}

impl Plugin for InterpreterPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_event::<Message>()
            .add_event::<Input>()
            .add_event::<BeginHalt>()
            .add_event::<HaltCountUp>()
            .add_event::<CubeState>()
            .add_event::<SolvedGoto>()
            .add_event::<ExecutedInstruction>()
            .add_event::<BeganProgram>()
            .add_event::<FinishedProgram>()
            .add_systems(
                Startup,
                if self.robot {
                    setup::<Cube3Robot>
                } else {
                    setup::<SimulatedPuzzle>
                },
            )
            .add_systems(Update, read_events);
    }
}

#[expect(clippy::too_many_arguments)]
fn read_events(
    recv: Res<EventRx>,
    mut messages: EventWriter<Message>,
    mut inputs: EventWriter<Input>,
    mut begin_halts: EventWriter<BeginHalt>,
    mut halt_count_ups: EventWriter<HaltCountUp>,
    mut cube_states: EventWriter<CubeState>,
    mut solved_gotos: EventWriter<SolvedGoto>,
    mut executed_instructions: EventWriter<ExecutedInstruction>,
    mut began_programs: EventWriter<BeganProgram>,
    mut finished_programs: EventWriter<FinishedProgram>,
) {
    for event in recv.try_iter() {
        match event {
            InterpretationEvent::Message(msg) => {
                messages.write(Message(msg));
            }
            InterpretationEvent::Input(int) => {
                inputs.write(Input(int));
            }
            InterpretationEvent::BeginHalt => {
                begin_halts.write(BeginHalt);
            }
            InterpretationEvent::HaltCountUp(int) => {
                halt_count_ups.write(HaltCountUp(int));
            }
            InterpretationEvent::CubeState(permutation) => {
                cube_states.write(CubeState(permutation));
            }
            InterpretationEvent::SolvedGoto { facelets } => {
                solved_gotos.write(SolvedGoto { facelets });
            }
            InterpretationEvent::ExecutedInstruction { next_one } => {
                executed_instructions.write(ExecutedInstruction { next_one });
            }
            InterpretationEvent::BeganProgram(intern) => {
                began_programs.write(BeganProgram(intern));
            }
            InterpretationEvent::FinishedProgram => {
                finished_programs.write(FinishedProgram);
            }
        }
    }
}
