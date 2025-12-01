// use bevy::{app::{App, Plugin, Startup, Update}, ecs::system::Commands};
use bevy::prelude::*;
use bevy_simple_text_input::{TextInput, TextInputSubmitEvent, TextInputValue};
use internment::Intern;
use itertools::Itertools;

use crate::interpreter_plugin::{
    BeganProgram, CommandTx, FinishedProgram, GaveInput, Input, InterpretationCommand, Message,
};

use super::interpreter_plugin::DoneExecuting;

const STEPPING: &str = "Manual stepping";
const AUTOMATIC: &str = "Automatic stepping";

pub struct IOViz;

#[derive(Component)]
struct ExecuteIndicator;

#[derive(Component)]
struct ExecuteIndicatorBg;

#[derive(Component)]
struct MessageDisplay;

#[derive(Component)]
struct MessageBox;

#[derive(Resource, Debug)]
// struct ExecuteClicked(bool);
enum ExecuteButtonState {
    None,
    Clicked,
    WaitingForInput,
}

impl Plugin for IOViz {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .insert_resource(ExecuteButtonState::None)
            .add_systems(Update, keyboard_control)
            .add_systems(Update, (started_program, got_message).chain())
            .add_systems(Update, on_submit)
            .add_systems(Update, step_on_input)
            .add_systems(Update, (finished_program, execute_conditionally).chain());
    }
}

fn setup(mut commands: Commands, window: Single<&Window>) {
    let panel = commands
        .spawn((
            Node {
                width: Val::Vw(25.),
                height: Val::Vh(100.),
                position_type: PositionType::Absolute,
                // display: Display::Flex,
                flex_direction: FlexDirection::Column,
                top: Val::Px(0.),
                left: Val::Px(0.),
                padding: UiRect::all(Val::Px(8.)),
                ..Default::default()
            },
            // BackgroundColor(Color::Srgba(GRAY)),
        ))
        .id();

    commands
        .spawn((
            Node {
                // flex_grow: 1.0,
                border: UiRect::all(Val::Px(4.)),
                height: Val::Vh(7.),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            BorderColor(Color::BLACK),
            BackgroundColor(Color::srgba(0., 0.6, 0., 0.5)),
            ExecuteIndicatorBg,
            ChildOf(panel),
        ))
        .with_child((
            Text(STEPPING.to_string()),
            TextLayout {
                justify: JustifyText::Center,
                ..Default::default()
            },
            TextFont {
                font_size: window.size().x / 66.,
                ..Default::default()
            },
            ExecuteIndicator,
        ));

    commands
        .spawn((
            Node {
                flex_grow: 1.,
                ..Default::default()
            },
            MessageBox,
            ChildOf(panel),
        ))
        .with_child((
            Text::new(String::new()),
            TextFont {
                font_size: window.size().x / 66.,
                ..Default::default()
            },
            MessageDisplay,
        ));

    let bottom_stuff = commands
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(2.0)),
                ..Default::default()
            },
            BorderColor(Color::WHITE),
            ChildOf(panel),
        ))
        .id();

    commands.spawn((TextInput, ChildOf(bottom_stuff)));
}

fn started_program(
    mut began_programs: EventReader<BeganProgram>,
    mut message_display: Single<&mut Text, With<MessageDisplay>>,
) {
    let Some(_) = began_programs.read().last() else {
        return;
    };

    **message_display = Text(String::new());
}

fn got_message(
    mut messages: EventReader<Message>,
    mut message_display: Single<(&mut Text, &TextFont), With<MessageDisplay>>,
    message_box: Single<&ComputedNode, With<MessageBox>>,
    window: Single<&Window>,
) {
    for message in messages.read() {
        #[expect(clippy::cast_precision_loss)]
        let height =
            (message_display.0.0.lines().count() + 1) as f32 * message_display.1.font_size * 1.2;
        #[expect(clippy::cast_precision_loss)]
        if height > message_box.size().y / window.physical_height() as f32 * window.height() {
            **message_display.0 = message_display.0.lines().skip(1).join("\n");
        }

        message_display.0.push('\n');
        message_display.0.push_str(&message.0);
    }
}

fn on_submit(
    mut submissions: EventReader<TextInputSubmitEvent>,
    command_tx: Res<CommandTx>,
    mut messages_tx: EventWriter<Message>,
) {
    for submission in submissions.read() {
        command_tx
            .send(InterpretationCommand::GiveInput(
                if let Ok(v) = submission.value.parse() {
                    messages_tx.write(Message(submission.value.clone()));
                    v
                } else {
                    messages_tx.write(Message("Input needs to be a number".to_owned()));
                    continue;
                },
            ))
            .unwrap();
    }
}

fn keyboard_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    command_tx: Res<CommandTx>,
    mut input: Single<&mut TextInputValue>,
    mut execute_button_state: ResMut<ExecuteButtonState>,
    mut text: Single<&mut Text, With<ExecuteIndicator>>,
    mut bg: Single<&mut BackgroundColor, With<ExecuteIndicatorBg>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyS) {
        command_tx
            .send(InterpretationCommand::Execute(Intern::from("simple")))
            .unwrap();

        command_tx
            .send(InterpretationCommand::Step)
            .unwrap();
    }

    if keyboard_input.just_pressed(KeyCode::KeyA) {
        command_tx
            .send(InterpretationCommand::Execute(Intern::from("avg")))
            .unwrap();

        command_tx
            .send(InterpretationCommand::Step)
            .unwrap();
    }

    if keyboard_input.just_pressed(KeyCode::KeyF) {
        command_tx
            .send(InterpretationCommand::Execute(Intern::from("fib")))
            .unwrap();

        command_tx
            .send(InterpretationCommand::Step)
            .unwrap();
    }

    if keyboard_input.just_pressed(KeyCode::KeyM) {
        command_tx
            .send(InterpretationCommand::Execute(Intern::from("multiply")))
            .unwrap();

        command_tx
            .send(InterpretationCommand::Step)
            .unwrap();
    }

    if keyboard_input.just_pressed(KeyCode::KeyE) {
        *execute_button_state = match *execute_button_state {
            ExecuteButtonState::None => {
                command_tx.send(InterpretationCommand::Step).unwrap();
                ***text = AUTOMATIC.to_string();
                bg.0 = Color::srgba(0.8, 0., 0., 0.5);
                ExecuteButtonState::Clicked
            }
            ExecuteButtonState::Clicked | ExecuteButtonState::WaitingForInput => {
                ***text = STEPPING.to_string();
                bg.0 = Color::srgba(0., 0.6, 0., 0.5);
                ExecuteButtonState::None
            }
            };
    }

    if keyboard_input.just_pressed(KeyCode::ArrowRight) {
        command_tx.send(InterpretationCommand::Step).unwrap();
    }

    input.0.retain(|c| c.is_ascii_digit());
}

fn step_on_input(
    command_tx: Res<CommandTx>,
    mut gave_inputs: EventReader<GaveInput>,
) {
    for _ in gave_inputs.read() {
        command_tx.send(InterpretationCommand::Step).unwrap();
    }
}

fn execute_conditionally(
    command_tx: Res<CommandTx>,
    mut execute_button_state: ResMut<ExecuteButtonState>,
    gave_inputs: EventReader<GaveInput>,
    inputs: EventReader<Input>,
    mut finished_instruction: EventReader<DoneExecuting>,
) {
    if let ExecuteButtonState::WaitingForInput = *execute_button_state {
        if !gave_inputs.is_empty() {
            *execute_button_state = ExecuteButtonState::Clicked;
        }
    } else if finished_instruction.read().last().is_none() {
        return;
    }

    match *execute_button_state {
        ExecuteButtonState::None | ExecuteButtonState::WaitingForInput => {}
        ExecuteButtonState::Clicked => {
            if inputs.is_empty() {
                command_tx.send(InterpretationCommand::Step).unwrap();
            } else {
                *execute_button_state = ExecuteButtonState::WaitingForInput;
            }
        }
    }
}

fn finished_program(
    mut execute_button_state: ResMut<ExecuteButtonState>,
    mut finished_programs: EventReader<FinishedProgram>,
    mut text: Single<&mut Text, With<ExecuteIndicator>>,
    mut bg: Single<&mut BackgroundColor, With<ExecuteIndicatorBg>>,
) {
    if finished_programs.read().last().is_some() {
        *execute_button_state = ExecuteButtonState::None;
        STEPPING.clone_into(&mut text);
        bg.0 = Color::srgba(0., 0.6, 0., 0.5);
    }
}
