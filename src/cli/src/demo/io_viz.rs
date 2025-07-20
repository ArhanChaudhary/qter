// use bevy::{app::{App, Plugin, Startup, Update}, ecs::system::Commands};
use bevy::{
    color::palettes::css::{GRAY, RED, YELLOW},
    prelude::*,
    text::FontStyle,
};

pub struct IOViz;

#[derive(Component)]
pub struct ChooserButton;

#[derive(Component)]
pub struct StepButton;

#[derive(Component)]
pub struct SolveButton;

#[derive(Component)]
pub struct ExecuteButton;

#[derive(Component)]
pub struct InputButton;

impl Plugin for IOViz {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (step_button, solve_button, execute_button, input_button).chain(),
        );
    }
}

fn setup(mut commands: Commands) {
    let panel = commands
        .spawn((
            Node {
                width: Val::Vw(20.),
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
                ..Default::default()
            },
            // BackgroundColor(Color::Srgba(RED)),
            ChildOf(panel),
        ))
        .id();

    commands.spawn((
        Text("Choose a program:".to_string()),
        TextLayout {
            justify: JustifyText::Center,
            ..Default::default()
        },
        ChildOf(choose_a_program),
    ));

    let chooser_buttons = commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Vh(20.),
                display: Display::Grid,
                grid_template_columns: vec![GridTrack::flex(1.0); 2],
                grid_template_rows: vec![GridTrack::flex(1.0); 2],
                align_items: AlignItems::Center,
                ..Default::default()
            },
            // BackgroundColor(Color::srgba(1., 1., 0., 0.5)),
            ChildOf(panel),
        ))
        .id();

    for program_choice in ["Simple", "Average", "Fibonacci", "Multiply"] {
        commands.spawn((
            Text(program_choice.to_string()),
            TextLayout {
                justify: JustifyText::Center,
                ..Default::default()
            },
            ChooserButton,
            BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.5)),
            ChildOf(chooser_buttons),
        ));
    }
}

fn step_button(mut commands: Commands, query: Query<Entity, With<StepButton>>) {}

fn solve_button(mut commands: Commands, query: Query<Entity, With<SolveButton>>) {}

fn execute_button(mut commands: Commands, query: Query<Entity, With<ExecuteButton>>) {}

fn input_button(mut commands: Commands, query: Query<Entity, With<InputButton>>) {}
