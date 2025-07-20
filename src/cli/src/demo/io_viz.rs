// use bevy::{app::{App, Plugin, Startup, Update}, ecs::system::Commands};
use bevy::prelude::*;
use bevy_simple_text_input::{TextInput, TextInputSubmitEvent};
use internment::Intern;
use itertools::Itertools;

use crate::demo::interpreter_plugin::{
    BeganProgram, CommandTx, FinishedProgram, GaveInput, Input, InterpretationCommand, Message,
};

pub struct IOViz;

#[derive(Component)]
pub struct InputButton(String, Intern<str>);

#[derive(Component)]
pub struct StepButton;

#[derive(Component)]
pub struct SolveButton;

#[derive(Component)]
pub struct ExecuteButton;

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
            .add_systems(
                Update,
                (step_button, solve_button, execute_button, input_button).chain(),
            )
            .add_systems(Update, (started_program, got_message).chain())
            .add_systems(Update, on_submit)
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

    let choose_a_program = commands
        .spawn((
            Node {
                justify_content: JustifyContent::Center,
                width: Val::Percent(100.),
                margin: UiRect::bottom(Val::Vh(2.)),
                ..Default::default()
            },
            // BackgroundColor(Color::Srgba(RED)),
            ChildOf(panel),
        ))
        .id();

    commands.spawn((
        Text("Choose a qter program:".to_string()),
        TextLayout {
            justify: JustifyText::Center,
            ..Default::default()
        },
        TextFont {
            font_size: window.size().x / 66.,
            ..Default::default()
        },
        ChildOf(choose_a_program),
    ));

    let input_buttons = commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Vh(20.),
                display: Display::Grid,
                column_gap: Val::Px(6.),
                row_gap: Val::Px(6.),
                grid_template_columns: vec![GridTrack::flex(1.0); 2],
                grid_template_rows: vec![GridTrack::flex(1.0); 2],
                // align_items: AlignItems::Center,
                ..Default::default()
            },
            // BackgroundColor(Color::srgba(1., 1., 0., 0.5)),
            ChildOf(panel),
        ))
        .id();

    for input_button in [
        InputButton("Simple".to_string(), Intern::<str>::from("simple")),
        InputButton("Average".to_string(), Intern::<str>::from("avg")),
        InputButton("Fibonacci".to_string(), Intern::<str>::from("fib")),
        InputButton("Multiply".to_string(), Intern::<str>::from("multiply")),
    ] {
        let text = input_button.0.clone();
        commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    display: Display::Flex,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                ChildOf(input_buttons),
                BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.5)),
                Button,
                input_button,
            ))
            .with_child((
                Text(text),
                TextLayout {
                    justify: JustifyText::Center,
                    ..Default::default()
                },
                TextFont {
                    font_size: window.size().x / 66.,
                    ..Default::default()
                },
            ));
    }

    commands
        .spawn((
            Node {
                margin: UiRect::top(Val::Vh(4.)),
                // flex_grow: 1.0,
                border: UiRect::all(Val::Px(4.)),
                height: Val::Vh(7.),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            BorderColor(Color::BLACK),
            BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.5)),
            Button,
            ExecuteButton,
            ChildOf(panel),
        ))
        .with_child((
            Text("Execute".to_string()),
            TextLayout {
                justify: JustifyText::Center,
                ..Default::default()
            },
            TextFont {
                font_size: window.size().x / 66.,
                ..Default::default()
            },
            ExecuteButton,
        ));

    commands
        .spawn((
            Node {
                margin: UiRect::top(Val::Vh(4.)),
                // flex_grow: 1.0,
                border: UiRect::all(Val::Px(4.)),
                height: Val::Vh(7.),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            BorderColor(Color::BLACK),
            BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.5)),
            Button,
            StepButton,
            ChildOf(panel),
        ))
        .with_child((
            Text("Step".to_string()),
            TextLayout {
                justify: JustifyText::Center,
                ..Default::default()
            },
            TextFont {
                font_size: window.size().x / 66.,
                ..Default::default()
            },
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
                    messages_tx.write(Message("Value needs to be parsable as a string".to_owned()));
                    continue;
                },
            ))
            .unwrap();
    }
}

fn input_button(
    interaction_query: Query<(&Interaction, &InputButton), Changed<Interaction>>,
    command_tx: Res<CommandTx>,
) {
    for (&interaction, input_button) in interaction_query {
        if interaction == Interaction::Pressed {
            command_tx
                .send(InterpretationCommand::Execute(input_button.1))
                .unwrap();
        }
    }
}

fn step_button(
    interaction_query: Query<(&Interaction, &StepButton), Changed<Interaction>>,
    command_tx: Res<CommandTx>,
) {
    for (&interaction, _) in interaction_query {
        if interaction == Interaction::Pressed {
            command_tx.send(InterpretationCommand::Step).unwrap();
        }
    }
}

fn solve_button(
    interaction_query: Query<&Interaction, With<SolveButton>>,
    command_tx: Res<CommandTx>,
) {
    if let Ok(&Interaction::Pressed) = interaction_query.single() {
        command_tx.send(InterpretationCommand::Solve).unwrap();
    }
}

fn execute_button(
    interaction: Single<&Interaction, (Changed<Interaction>, With<ExecuteButton>)>,
    mut execute_button_state: ResMut<ExecuteButtonState>,
    mut text: Single<&mut Text, With<ExecuteButton>>,
) {
    // if let Ok(&Interaction::Pressed) = interaction_query.single() {
    if *interaction != &Interaction::Pressed {
        return;
    }
    // dbg!(&children.0);
    *execute_button_state = match *execute_button_state {
        ExecuteButtonState::None => {
            ***text = "Pause".to_string();
            ExecuteButtonState::Clicked
        }
        ExecuteButtonState::Clicked => {
            ***text = "Execute".to_string();
            ExecuteButtonState::None
        }
        ExecuteButtonState::WaitingForInput => ExecuteButtonState::WaitingForInput,
    };
}

fn execute_conditionally(
    command_tx: Res<CommandTx>,
    mut execute_button_state: ResMut<ExecuteButtonState>,
    gave_inputs: EventReader<GaveInput>,
    inputs: EventReader<Input>,
) {
    match *execute_button_state {
        ExecuteButtonState::None => (),
        ExecuteButtonState::WaitingForInput => {
            if !gave_inputs.is_empty() {
                *execute_button_state = ExecuteButtonState::Clicked;
            }
        }
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
    mut text: Single<&mut Text, With<ExecuteButton>>,
) {
    if finished_programs.read().last().is_some() {
        *execute_button_state = ExecuteButtonState::None;
        "Execute".clone_into(&mut text);
    }
}
