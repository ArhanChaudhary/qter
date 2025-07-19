use bevy::prelude::*;

use super::{
    PROGRAMS,
    interpreter_plugin::{BeganProgram, ExecutingInstruction},
};

pub struct CodeViz;

impl Plugin for CodeViz {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (started_program, next_instruction).chain());
    }
}

#[derive(Component)]
struct Code;

#[derive(Component)]
struct Highlight;

fn setup(mut commands: Commands, window: Single<&Window>) {
    let panel = commands
        .spawn((
            Node {
                // width: Val::Vw(33.),
                height: Val::Vh(100.),
                position_type: PositionType::Absolute,
                top: Val::Px(0.),
                left: Val::Vw(33.5),
                padding: UiRect::all(Val::Px(8.)),
                ..Default::default()
            },
            // BackgroundColor(Color::srgba_u8(128, 128, 255, 128)),
        ))
        .id();

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.),
            top: Val::Px(0.),
            height: Val::Px(0.),
            padding: UiRect::right(Val::Px(8.)),
            box_sizing: BoxSizing::ContentBox,
            ..Default::default()
        },
        BackgroundColor(Color::srgba_u8(255, 0, 255, 128)),
        Highlight,
        ChildOf(panel),
    ));

    commands.spawn((
        Text(String::new()),
        TextFont {
            font_size: window.size().x / 66.,
            ..Default::default()
        },
        Code,
        ChildOf(panel),
    ));
}

fn started_program(
    mut began_programs: EventReader<BeganProgram>,
    mut code: Single<(&mut Text, &Code)>,
) {
    let Some(program) = began_programs.read().last() else {
        return;
    };

    *code.0 = Text(PROGRAMS.get(&program.0).unwrap().code.clone());
}

#[expect(clippy::cast_precision_loss)]
fn next_instruction(
    mut executing_instructions: EventReader<ExecutingInstruction>,
    code: Single<(&Text, &TextFont, &Code)>,
    mut highlight: Single<(&mut Node, &Highlight)>,
) {
    let Some(instruction) = executing_instructions.read().last() else {
        return;
    };

    let target_lineno = instruction.which_one.to_string();

    let text_size = code.1.font_size;
    let code = &code.0.0;

    let mut lines = code.split('\n').enumerate();

    let (idx, _) = lines
        .by_ref()
        .find(|(_, line)| line.starts_with(&target_lineno))
        .unwrap();

    let end = lines
        .by_ref()
        .find(|(_, line)| line.is_empty() || line.contains('|') || line.contains("--"))
        .map_or_else(|| code.split('\n').count(), |(idx, _)| idx);

    highlight.0.top = Val::Px(text_size * 1.2 * idx as f32 + 8.);
    highlight.0.height = Val::Px(text_size * 1.2 * (end - idx) as f32);
}
