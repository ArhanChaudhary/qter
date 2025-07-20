// use bevy::{app::{App, Plugin, Startup, Update}, ecs::system::Commands};
use bevy::{
    color::palettes::css::{GRAY, RED, YELLOW},
    prelude::*,
    text::FontStyle,
};
use bevy_simple_text_input::{TextInput, TextInputSubmitEvent};
use internment::Intern;
use qter_core::Int;

use crate::demo::interpreter_plugin::{BeganProgram, CommandTx, InterpretationCommand, Message};

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

impl Plugin for IOViz {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(
                Update,
                (step_button, solve_button, execute_button, input_button).chain(),
            )
            .add_systems(Update, (started_program, got_message).chain())
            .add_systems(Update, on_submit);
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
                flex_grow: 1.,
                ..Default::default()
            },
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
    mut message_display: Single<&mut Text, With<MessageDisplay>>,
) {
    for message in messages.read() {
        message_display.0.push('\n');
        message_display.0.push_str(&message.0);
    }
}

fn on_submit(
    mut submissions: EventReader<TextInputSubmitEvent>,
    command_tx: Res<CommandTx>,
    mut message_display: Single<&mut Text, With<MessageDisplay>>,
) {
    for submission in submissions.read() {
        command_tx
            .send(InterpretationCommand::GiveInput(
                if let Ok(v) = submission.value.parse() {
                    message_display.push('\n');
                    message_display.push_str(&submission.value);
                    v
                } else {
                    message_display.push_str("\nValue needs to be parsable as a string");
                    continue;
                },
            ))
            .unwrap();
    }
}

// fn step_button(mut commands: Commands, query: Query<Entity, With<StepButton>>) {}

// fn solve_button(mut commands: Commands, query: Query<Entity, With<SolveButton>>) {}

// fn execute_button(mut commands: Commands, query: Query<Entity, With<ExecuteButton>>) {}

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
    interaction_query: Query<&Interaction, With<StepButton>>,
    command_tx: Res<CommandTx>,
) {
    if let Ok(&Interaction::Pressed) = interaction_query.single() {
        command_tx.send(InterpretationCommand::Step).unwrap();
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
    interaction_query: Query<&Interaction, With<ExecuteButton>>,
    command_tx: Res<CommandTx>,
) {
    if let Ok(&Interaction::Pressed) = interaction_query.single() {
        command_tx
            .send(InterpretationCommand::Execute(Intern::from("multiply")))
            .unwrap();
    }
}

/*
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

    if keyboard_input.just_pressed(KeyCode::Enter) {
        command_tx
            .send(InterpretationCommand::GiveInput(Int::one()))
            .unwrap();
    }
}
*/
