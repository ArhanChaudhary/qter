use bevy::prelude::*;

use super::{PROGRAMS, interpreter_plugin::BeganProgram};

pub struct CodeViz;

impl Plugin for CodeViz {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, started_program);
    }
}

#[derive(Component)]
struct Code;

#[derive(Component)]
struct Highlight;

fn setup(mut commands: Commands) {
    let panel = commands
        .spawn((
            Node {
                // width: Val::Vw(33.),
                height: Val::Vh(100.),
                position_type: PositionType::Absolute,
                top: Val::Px(0.),
                left: Val::Vw(33.5),
                ..Default::default()
            },
            BackgroundColor(Color::srgba_u8(128, 128, 255, 128)),
        ))
        .id();

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.),
            top: Val::Px(90. * 1.2),
            height: Val::Px(60.0 * 1.2),
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
            font_size: 30.,
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
