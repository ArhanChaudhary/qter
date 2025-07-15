use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, LazyLock},
    thread,
};

use bevy::prelude::*;
use chumsky::container::Seq;
use compiler::compile;
use crossbeam_channel::{Receiver, Sender, unbounded};
use internment::{ArcIntern, Intern};
use interpreter::puzzle_states::SimulatedPuzzle;
use interpreter_loop::{CUBE3, CUBE3_DEF};
use qter_core::{
    Facelets, File, I, Int, Program, U,
    architectures::{Architecture, Permutation},
};

use crate::robot::{Cube3Robot, RobotLike};

mod interpreter_loop;

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

static NAMES: &[&str] = &["A", "B", "C", "D"];

#[derive(Event)]
struct Message(String);

#[derive(Event)]
struct Input(Int<U>);

#[derive(Event)]
struct BeginHalt;

#[derive(Event)]
struct HaltCountUp(Int<U>);

#[derive(Event)]
struct CubeState(Permutation);

#[derive(Event)]
struct SolvedGoto {
    facelets: Facelets,
}

#[derive(Event)]
struct ExecutedInstruction {
    next_one: usize,
}

#[derive(Event)]
struct BeganProgram(Intern<str>);

#[derive(Event)]
struct FinishedProgram;

#[derive(Debug)]
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

#[derive(Component)]
struct FaceletIdx(usize);

#[derive(Component)]
struct StateViz;

#[derive(Component)]
struct CycleViz;

#[derive(Component)]
struct Border;

#[derive(Component)]
struct Sticker;

#[derive(Component)]
struct SolvedGotoStatement;

#[derive(Component)]
struct RegistersViz;

#[derive(Resource)]
struct Colors(HashMap<ArcIntern<str>, Handle<ColorMaterial>>);

#[derive(Resource)]
struct CurrentState(Permutation);

fn setup<R: RobotLike + Send + 'static>(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    let (event_tx, event_rx) = unbounded::<InterpretationEvent>();
    let (command_tx, command_rx) = unbounded::<InterpretationCommand>();

    thread::spawn(move || interpreter_loop::interpreter_loop::<R>(event_tx, command_rx));

    commands.insert_resource(EventRx(event_rx));
    commands.insert_resource(CommandTx(command_tx));
    commands.insert_resource(CurrentState(CUBE3.identity()));

    let scale = 35.;

    let weird_dist = (3_f32 / 4.).sqrt() * scale * 2.;

    let rhombus_matrix = Mat2::from_diagonal(Vec2::new(weird_dist, scale))
        * Mat2::from_cols(Vec2::new(-1., 1.), Vec2::new(1., 1.));

    let sticker = meshes.add(Rhombus::new(weird_dist * 2. * 0.9, 2. * scale * 0.9));
    let border = meshes.add(Rhombus::new(weird_dist * 2. * 1.1, 2. * scale * 1.1));

    let dist = 450.;
    let off_center = Vec2::new(350., -150.);

    let spots = [(false, false), (false, true), (true, false), (true, true)];

    let indices = [
        0, 1, 2, 3, 4, 5, 6, 7, //
        21, 19, 16, 22, 17, 23, 20, 18, //
        31, 30, 29, 28, 27, 26, 25, 24, //
        32, 33, 34, 35, 36, 37, 38, 39, //
        42, 44, 47, 41, 46, 40, 43, 45, //
        10, 12, 15, 9, 14, 8, 11, 13, //
    ];

    let mut colors = HashMap::new();

    colors.insert(
        ArcIntern::from("White"),
        materials.add(Color::srgb_u8(255, 255, 255)),
    );
    colors.insert(
        ArcIntern::from("Green"),
        materials.add(Color::srgb_u8(0, 255, 0)),
    );
    colors.insert(
        ArcIntern::from("Red"),
        materials.add(Color::srgb_u8(255, 0, 0)),
    );
    colors.insert(
        ArcIntern::from("Blue"),
        materials.add(Color::srgb_u8(0, 0, 255)),
    );
    colors.insert(
        ArcIntern::from("Orange"),
        materials.add(Color::srgb_u8(255, 128, 0)),
    );
    colors.insert(
        ArcIntern::from("Yellow"),
        materials.add(Color::srgb_u8(255, 255, 0)),
    );
    colors.insert(
        ArcIntern::from("Grey"),
        materials.add(Color::srgb_u8(127, 127, 127)),
    );
    colors.insert(
        ArcIntern::from("Purple"),
        materials.add(Color::srgb_u8(255, 0, 255)),
    );
    colors.insert(
        ArcIntern::from("Transparent"),
        materials.add(Color::srgba_u8(0, 0, 0, 0)),
    );

    let grey = ArcIntern::from("Grey");
    let transparent = colors.get(&ArcIntern::from("Transparent")).unwrap();

    let center_colors = [
        ArcIntern::<str>::from("White"),
        ArcIntern::from("Green"),
        ArcIntern::from("Red"),
        ArcIntern::from("Blue"),
        ArcIntern::from("Yellow"),
        ArcIntern::from("Orange"),
    ];

    for (is_cycle_viz, is_right) in spots {
        let mut transform =
            Mat4::from_translation(Vec3::new(off_center.x, dist / 2. + off_center.y, 0.));

        if !is_cycle_viz {
            transform *= Mat4::from_translation(Vec3::new(0., -dist, 0.));
        }

        let idx_to_add = if is_right { 3 } else { 0 };

        if is_right {
            transform *= Mat4::from_translation(Vec3::new(dist, 0., 0.))
                * Mat4::from_rotation_z((60_f32).to_radians());
        }

        let tri_translate = Mat4::from_translation(Vec3::new(0., scale * 3., 0.));

        for (j, tri) in [
            tri_translate,
            Mat4::from_rotation_z((120_f32).to_radians()) * tri_translate,
            Mat4::from_rotation_z((240_f32).to_radians()) * tri_translate,
        ]
        .into_iter()
        .enumerate()
        {
            for (i, (x, y)) in [
                (1., 1.),
                (0., 1.),
                (-1., 1.),
                (1., 0.),
                (-1., 0.),
                (1., -1.),
                (0., -1.),
                (-1., -1.),
                (0., 0.),
            ]
            .into_iter()
            .enumerate()
            {
                let spot = rhombus_matrix * Vec2::new(x, y);
                let transform =
                    transform * tri * Mat4::from_translation(Vec3::new(spot.x, spot.y, 0.));

                let color = colors
                    .get(if !is_cycle_viz || i == 8 {
                        &center_colors[j + idx_to_add]
                    } else {
                        &grey
                    })
                    .unwrap()
                    .clone();

                if i == 8 {
                    commands.spawn((
                        Mesh2d(sticker.clone()),
                        MeshMaterial2d(color),
                        Transform::from_matrix(transform),
                    ));
                } else {
                    let facelet_idx = indices[(j + idx_to_add) * 8 + i];

                    if is_cycle_viz {
                        commands.spawn((
                            Mesh2d(border.clone()),
                            MeshMaterial2d(transparent.clone()),
                            Transform::from_matrix(
                                Mat4::from_translation(Vec3::new(0., 0., -1.)) * transform,
                            ),
                            FaceletIdx(facelet_idx),
                            CycleViz,
                            Border,
                        ));

                        commands.spawn((
                            Mesh2d(sticker.clone()),
                            MeshMaterial2d(color),
                            Transform::from_matrix(transform),
                            FaceletIdx(facelet_idx),
                            CycleViz,
                            Sticker,
                        ));
                    } else {
                        commands.spawn((
                            Mesh2d(border.clone()),
                            MeshMaterial2d(transparent.clone()),
                            Transform::from_matrix(
                                Mat4::from_translation(Vec3::new(0., 0., -1.)) * transform,
                            ),
                            FaceletIdx(facelet_idx),
                            StateViz,
                            Border,
                        ));

                        commands.spawn((
                            Mesh2d(sticker.clone()),
                            MeshMaterial2d(color),
                            Transform::from_matrix(transform),
                            FaceletIdx(facelet_idx),
                            StateViz,
                            Sticker,
                        ));
                    }

                    // commands.spawn((
                    //     Text2d::new(facelet_idx.to_string()),
                    //     TextColor(Color::srgb_u8(0, 0, 0)),
                    //     Transform::from_matrix(transform).with_rotation(Quat::IDENTITY),
                    // ));
                }
            }
        }
    }

    commands.insert_resource(Colors(colors));
}

pub fn demo(robot: bool) {
    let mut app = App::new();
    let app = app
        .add_event::<Message>()
        .add_event::<Input>()
        .add_event::<BeginHalt>()
        .add_event::<HaltCountUp>()
        .add_event::<CubeState>()
        .add_event::<SolvedGoto>()
        .add_event::<ExecutedInstruction>()
        .add_event::<BeganProgram>()
        .add_event::<FinishedProgram>()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, keyboard_control)
        .add_systems(
            Update,
            (
                read_events,
                executed_instruction,
                state_visualizer,
                solved_goto_visualizer,
                finished_program,
            )
                .chain(),
        );

    if robot {
        app.add_systems(Startup, setup::<Cube3Robot>)
    } else {
        app.add_systems(Startup, setup::<SimulatedPuzzle>)
    }
    .run();
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

fn executed_instruction(
    mut commands: Commands,
    colors: Res<Colors>,
    mut executed_instructions: EventReader<ExecutedInstruction>,
    mut backgrounds: Query<(&mut MeshMaterial2d<ColorMaterial>, &StateViz, &Border)>,
    solved_goto_statements: Query<(Entity, &SolvedGotoStatement)>,
) {
    let Some(instr) = executed_instructions.read().last() else {
        return;
    };

    let transparent = colors.0.get(&ArcIntern::from("Transparent")).unwrap();

    backgrounds
        .par_iter_mut()
        .for_each(|(mut color, StateViz, Border)| *color = MeshMaterial2d(transparent.to_owned()));

    for (entity, SolvedGotoStatement) in solved_goto_statements {
        commands.entity(entity).despawn();
    }
}

fn state_visualizer(
    colors: Res<Colors>,
    mut current_state: ResMut<CurrentState>,
    mut cube_states: EventReader<CubeState>,
    mut query: Query<(
        &mut MeshMaterial2d<ColorMaterial>,
        &FaceletIdx,
        &StateViz,
        &Sticker,
    )>,
) {
    let Some(state) = cube_states.read().last() else {
        return;
    };

    state.0.clone_into(&mut current_state.0);

    query
        .par_iter_mut()
        .for_each(|(mut color_material, facelet, StateViz, Sticker)| {
            let new_color = colors
                .0
                .get(&CUBE3.facelet_colors()[state.0.mapping()[facelet.0]])
                .unwrap()
                .clone();

            *color_material = MeshMaterial2d(new_color);
        });
}

fn solved_goto_visualizer(
    mut commands: Commands,
    colors: Res<Colors>,
    current_state: Res<CurrentState>,
    mut solved_gotos: EventReader<SolvedGoto>,
    mut query: Query<(
        &mut MeshMaterial2d<ColorMaterial>,
        &FaceletIdx,
        &StateViz,
        &Border,
    )>,
) {
    let Some(solved_goto) = solved_gotos.read().last() else {
        return;
    };

    let purple = colors.0.get(&ArcIntern::from("Purple")).unwrap();

    let color_scheme = CUBE3.facelet_colors();

    let mut taken = true;

    for (mut color, idx, StateViz, Border) in &mut query {
        if solved_goto.facelets.0.contains(&idx.0) {
            *color = MeshMaterial2d(purple.to_owned());

            taken &= color_scheme[current_state.0.mapping()[idx.0]] == color_scheme[idx.0];
        }
    }

    if taken {
        commands.spawn((
            Text2d::new("Taken"),
            TextColor(Color::srgb_u8(0, 255, 0)),
            TextFont {
                font_size: 50.,
                ..Default::default()
            },
            Transform::from_translation(Vec3::new(300. + 250., 0., 0.)),
            SolvedGotoStatement,
        ));
    } else {
        commands.spawn((
            Text2d::new("Not taken"),
            TextColor(Color::srgb_u8(255, 0, 0)),
            TextFont {
                font_size: 50.,
                ..Default::default()
            },
            Transform::from_translation(Vec3::new(300. + 250., 0., 0.)),
            SolvedGotoStatement,
        ));
    }
}

fn finished_program(
    mut commands: Commands,
    mut executed_instructions: EventReader<FinishedProgram>,
    registers_viz: Query<(Entity, &RegistersViz)>,
) {
    let Some(FinishedProgram) = executed_instructions.read().last() else {
        return;
    };

    for (entity, RegistersViz) in registers_viz {
        commands.entity(entity).despawn();
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
