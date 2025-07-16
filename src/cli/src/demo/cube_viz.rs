use std::collections::HashMap;

use bevy::prelude::*;
use internment::ArcIntern;

use super::{
    CurrentState,
    interpreter_loop::CUBE3,
    interpreter_plugin::{CubeState, ExecutedInstruction, FinishedProgram, SolvedGoto},
};

pub struct CubeViz;

static NAMES: &[&str] = &["A", "B", "C", "D"];

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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    commands.insert_resource(CurrentState(CUBE3.identity()));

    let scale = 30.;

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

    commands
        .spawn((Node {
            display: Display::Grid,
            width: Val::Vw(33.),
            height: Val::Vh(100.),
            column_gap: Val::Px(0.),
            row_gap: Val::Px(0.),
            margin: UiRect::all(Val::Px(0.)),
            position_type: PositionType::Absolute,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceEvenly,
            grid_template_columns: vec![GridTrack::flex(1.), GridTrack::flex(1.)],
            grid_template_rows: vec![GridTrack::flex(1.), GridTrack::flex(1.)],
            top: Val::Px(0.),
            right: Val::Px(0.),
            ..Node::default()
        },))
        .with_children(|builder| {
            // These offsets are probably not responsive
            let center = Mat4::from_translation(Vec3::new(
                -scale * 2. * 9.3 * 2.,
                -scale * 2. * 6. * 2.,
                0.,
            ));

            for (is_cycle_viz, is_right) in spots {
                builder
                    .spawn((Node {
                        display: Display::Grid,
                        width: Val::Px(weird_dist * 2. * 3.),
                        height: Val::Px(scale * 2. * 6.),
                        margin: UiRect::all(Val::Px(0.)),
                        padding: UiRect::all(Val::Px(0.)),
                        ..Node::default()
                    },))
                    .with_children(|builder| {
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
                                let transform = center
                                    * tri
                                    * Mat4::from_translation(Vec3::new(spot.x, spot.y, 0.));

                                println!("{:?} {is_cycle_viz}", Transform::from_matrix(transform));

                                let color = colors
                                    .get(if !is_cycle_viz || i == 8 {
                                        &center_colors[j + idx_to_add]
                                    } else {
                                        &grey
                                    })
                                    .unwrap()
                                    .clone();

                                if i == 8 {
                                    builder.spawn((
                                        Mesh2d(sticker.clone()),
                                        MeshMaterial2d(color),
                                        Transform::from_matrix(transform),
                                    ));
                                } else {
                                    let facelet_idx = indices[(j + idx_to_add) * 8 + i];

                                    if is_cycle_viz {
                                        builder.spawn((
                                            Mesh2d(border.clone()),
                                            MeshMaterial2d(transparent.clone()),
                                            Transform::from_matrix(
                                                Mat4::from_translation(Vec3::new(0., 0., -1.))
                                                    * transform,
                                            ),
                                            FaceletIdx(facelet_idx),
                                            CycleViz,
                                            Border,
                                        ));

                                        builder.spawn((
                                            Mesh2d(sticker.clone()),
                                            MeshMaterial2d(color),
                                            Transform::from_matrix(transform),
                                            FaceletIdx(facelet_idx),
                                            CycleViz,
                                            Sticker,
                                        ));
                                    } else {
                                        builder.spawn((
                                            Mesh2d(border.clone()),
                                            MeshMaterial2d(transparent.clone()),
                                            Transform::from_matrix(
                                                Mat4::from_translation(Vec3::new(0., 0., -1.))
                                                    * transform,
                                            ),
                                            FaceletIdx(facelet_idx),
                                            StateViz,
                                            Border,
                                        ));

                                        builder.spawn((
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
                    });
            }
        });

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
                finished_program,
            )
                .chain(),
        );
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
