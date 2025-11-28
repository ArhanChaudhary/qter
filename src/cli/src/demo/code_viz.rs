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
struct Panel;

#[derive(Component)]
struct Highlight;

fn setup(mut commands: Commands, window: Single<&Window>) {
    let panel = commands
        .spawn((
            Node {
                // width: Val::Vw(33.),
                // height: Val::Vh(100.),
                position_type: PositionType::Absolute,
                top: Val::Px(0.),
                left: Val::Vw(26.),
                padding: UiRect::all(Val::Px(8.)),
                overflow: Overflow::visible(),
                ..Default::default()
            },
            Panel,
            // BackgroundColor(Color::srgba_u8(128, 128, 255, 128)),
        ))
        .id();

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.),
                top: Val::Px(0.),
                height: Val::Px(0.),
                padding: UiRect::right(Val::Px(8.)),
                //// box_sizing: BoxSizing::ContentBox,
                overflow: Overflow::visible(),
                ..Default::default()
            },
            BackgroundColor(Color::srgba_u8(255, 0, 255, 128)),
            Highlight,
        ))
        .set_parent(panel);

    commands
        .spawn((
            Text(String::new()),
            TextFont {
                font_size: window.size().x / 66.,
                ..Default::default()
            },
            Code,
        ))
        .set_parent(panel);
}

fn started_program(
    mut began_programs: EventReader<BeganProgram>,
    mut panel: Single<&mut Node, (With<Panel>, Without<Highlight>)>,
    mut code: Single<(&mut Text, &Code)>,
    mut highlight: Single<(&mut Node, &Highlight)>,
) {
    let Some(program) = began_programs.read().last() else {
        return;
    };

    *code.0 = Text(PROGRAMS.get(&program.0).unwrap().code.clone());

    highlight.0.height = Val::ZERO;
    panel.top = Val::ZERO;
}

#[expect(clippy::cast_precision_loss)]
fn next_instruction(
    mut executing_instructions: EventReader<ExecutingInstruction>,
    mut panel: Single<&mut Node, (With<Panel>, Without<Highlight>)>,
    code: Single<(&Text, &TextFont, &Code), Without<Highlight>>,
    mut highlight: Single<(&mut Node, &Highlight)>,
    window: Single<&Window>,
) {
    let Some(instruction) = executing_instructions.read().last() else {
        return;
    };

    let target_lineno = instruction.which_one.to_string();

    let text_size = code.1.font_size;
    let mut lines = code.0.0.split('\n').enumerate();

    let (idx, _) = lines
        .by_ref()
        .find(|(_, line)| line.starts_with(&target_lineno))
        .unwrap();

    let end = lines
        .by_ref()
        .find(|(_, line)| line.is_empty() || line.contains('|') || line.contains("--"))
        .map_or_else(|| code.0.0.split('\n').count(), |(idx, _)| idx);

    let start_spot = text_size * 1.2 * idx as f32 + 8.;
    let size = text_size * 1.2 * (end - idx) as f32;
    let end_spot = start_spot + size;

    highlight.0.top = Val::Px(start_spot);
    highlight.0.height = Val::Px(size);

    let offset = match panel.top {
        Val::Px(px) => px,
        Val::Auto => 0.,
        _ => unreachable!(),
    };

    if start_spot + offset < 0. {
        panel.top = Val::Px(-start_spot);
    }

    let max_spot = window.size().y * 9. / 10.;
    println!("{end_spot} {offset} {max_spot}");
    if end_spot + offset > max_spot {
        panel.top = Val::Px(max_spot - end_spot);
    }
}
