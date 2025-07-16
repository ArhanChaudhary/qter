use std::{collections::HashMap, sync::Arc};

use bevy::prelude::*;
use internment::ArcIntern;

use super::{
    CurrentState, PROGRAMS,
    interpreter_loop::CUBE3,
    interpreter_plugin::{
        BeganProgram, CubeState, ExecutedInstruction, FinishedProgram, SolvedGoto,
    },
};

pub struct CubeViz;

static NAMES: &[&str] = &["A", "B", "C", "D", "E", "F", "G"];

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

#[derive(Component)]
struct RegistersList;

#[derive(Resource)]
struct Colors(HashMap<ArcIntern<str>, Handle<ColorMaterial>>);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    commands.insert_resource(CurrentState(CUBE3.identity()));

    let scale = 35.;

    let weird_dist = (3_f32 / 4.).sqrt() * scale * 2.;

    let rhombus_matrix = Mat2::from_diagonal(Vec2::new(weird_dist, scale))
        * Mat2::from_cols(Vec2::new(-1., 1.), Vec2::new(1., 1.));

    let sticker = meshes.add(Rhombus::new(weird_dist * 2. * 0.9, 2. * scale * 0.9));
    let border = meshes.add(Rhombus::new(weird_dist * 2. * 1.1, 2. * scale * 1.1));

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

    let panel = commands
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                width: Val::Vw(33.),
                height: Val::Vh(100.),
                top: Val::Px(0.),
                right: Val::Px(0.),
                ..Default::default()
            },
            // BackgroundColor(Color::srgba_u8(128, 128, 255, 128)),
        ))
        .id();

    commands.spawn((
        Node {
            flex_grow: 1.,
            display: Display::Grid,
            align_content: AlignContent::Start,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Start,
            justify_items: JustifyItems::Start,
            column_gap: Val::Px(16.),
            ..Default::default()
        },
        // BackgroundColor(Color::srgba_u8(255, 128, 128, 128)),
        RegistersList,
        ChildOf(panel),
    ));

    let puzzles = commands
        .spawn((
            Node {
                display: Display::Grid,
                column_gap: Val::Px(0.),
                row_gap: Val::Px(0.),
                margin: UiRect::all(Val::Px(0.)),
                align_items: AlignItems::Center,
                align_content: AlignContent::SpaceEvenly,
                justify_items: JustifyItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                grid_template_columns: vec![GridTrack::flex(1.), GridTrack::flex(1.)],
                grid_template_rows: vec![GridTrack::flex(1.), GridTrack::flex(1.)],
                height: Val::Px(scale * 2. * 6. * 2. * 1.1),
                ..Node::default()
            },
            // BackgroundColor(Color::srgba_u8(128, 255, 128, 128)),
            ChildOf(panel),
        ))
        .id();

    // These offsets are hardcoded and probably not responsive
    let center = Mat4::from_translation(Vec3::new(
        -scale * 2. * 8.0 * 2.,
        -scale * 2. * 4.65 * 3.77,
        0.,
    ));
    // let center = Mat4::IDENTITY;

    for (is_cycle_viz, is_right) in spots {
        let puzzle = commands
            .spawn((
                Node {
                    display: Display::Grid,
                    width: Val::Px(weird_dist * 2. * 3.),
                    height: Val::Px(scale * 2. * 6.),
                    margin: UiRect::all(Val::Px(0.)),
                    padding: UiRect::all(Val::Px(0.)),
                    ..Node::default()
                },
                // BackgroundColor(Color::srgba_u8(128, 255, 255, 128)),
                ChildOf(puzzles),
            ))
            .id();
        // builder.spawn((
        //     Node {
        //         ..Default::default()
        //     },
        //     BackgroundColor(Color::srgba_u8(128, 0, 255, 127)),
        //     Text2d::new(format!("{is_cycle_viz}-{is_right}")),
        //     TextColor(Color::srgb_u8(128, 255, 255)),
        // ));

        let idx_to_add = if is_right { 3 } else { 0 };

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
                    center * tri * Mat4::from_translation(Vec3::new(spot.x, spot.y, 0.));

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
                        ChildOf(puzzle),
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
                            ChildOf(puzzle),
                        ));

                        commands.spawn((
                            Mesh2d(sticker.clone()),
                            MeshMaterial2d(color),
                            Transform::from_matrix(transform),
                            FaceletIdx(facelet_idx),
                            CycleViz,
                            Sticker,
                            ChildOf(puzzle),
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
                            ChildOf(puzzle),
                        ));

                        commands.spawn((
                            Mesh2d(sticker.clone()),
                            MeshMaterial2d(color),
                            Transform::from_matrix(transform),
                            FaceletIdx(facelet_idx),
                            StateViz,
                            Sticker,
                            ChildOf(puzzle),
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

impl Plugin for CubeViz {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                executed_instruction,
                state_visualizer,
                solved_goto_visualizer,
                started_program,
                finished_program,
            )
                .chain(),
        );
    }
}

fn started_program(
    mut commands: Commands,
    mut began_programs: EventReader<BeganProgram>,
    regs_list: Query<(Entity, &RegistersList)>,
) {
    let Some(program) = began_programs.read().last() else {
        return;
    };

    let Some((regs_list, RegistersList)) = regs_list.iter().next() else {
        unreachable!();
    };

    let arch = Arc::clone(&PROGRAMS.get(&program.0).unwrap().architecture);

    for (i, reg) in arch.registers().iter().enumerate() {
        #[expect(clippy::cast_possible_wrap)]
        #[expect(clippy::cast_possible_truncation)]
        let row = GridPlacement::start_span(i as i16 + 1, 1);

        commands
            .spawn((
                Node {
                    grid_column: GridPlacement::start_span(1, 1),
                    grid_row: row,
                    ..Default::default()
                },
                RegistersViz,
                ChildOf(regs_list),
            ))
            .with_child((
                Text::new(NAMES[i]),
                TextColor::WHITE,
                TextFont {
                    font_size: 80.,
                    ..Default::default()
                },
            ));

        commands
            .spawn((
                Node {
                    grid_column: GridPlacement::start_span(2, 1),
                    grid_row: row,
                    ..Default::default()
                },
                RegistersViz,
                ChildOf(regs_list),
            ))
            .with_child((
                Text::new(format!("= 0/{}  ", reg.order())),
                TextColor::WHITE,
                TextFont {
                    font_size: 40.,
                    ..Default::default()
                },
            ));

        for (j, cycle) in reg.unshared_cycles().iter().enumerate() {
            let cycle_box = commands
                .spawn((
                    Node {
                        #[expect(clippy::cast_possible_wrap)]
                        #[expect(clippy::cast_possible_truncation)]
                        grid_column: GridPlacement::start_span(j as i16 + 3, 1),
                        grid_row: row,
                        justify_self: JustifySelf::Stretch,
                        padding: UiRect::all(Val::Px(4.)),
                        display: Display::Grid,
                        ..Default::default()
                    },
                    RegistersViz,
                    #[expect(clippy::cast_precision_loss)]
                    BackgroundColor(Color::oklch(0.76, 0.12, j as f32 * 360. / 1.61)),
                    ChildOf(regs_list),
                ))
                .id();

            let text_container = commands
                .spawn((
                    Node {
                        justify_self: JustifySelf::Center,
                        ..Default::default()
                    },
                    ChildOf(cycle_box),
                ))
                .id();

            commands.spawn((
                Text::new(format!("0/{}", cycle.chromatic_order())),
                TextColor::WHITE,
                TextFont {
                    font_size: 40.,
                    ..Default::default()
                },
                TextLayout::new_with_justify(JustifyText::Center),
                ChildOf(text_container),
            ));
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
    let Some(_) = executed_instructions.read().last() else {
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
            // Transform::from_translation(Vec3::new(spot.0.x + 250., spot.0.y, 0.)),
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
            Transform::from_translation(Vec3::new(350. + 250., -150., 0.)),
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
