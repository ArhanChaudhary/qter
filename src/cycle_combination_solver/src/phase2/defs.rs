use super::puzzle::{Cube3Storage, Move, OrbitDef, PuzzleDef, PuzzleState, PuzzleStateCore};
use std::sync::LazyLock;

pub static CUBE3_DEF: LazyLock<PuzzleDef<Cube3Storage>> = LazyLock::new(|| {
    let orbit_defs = vec![
        OrbitDef {
            name: "edges".to_owned(),
            size: 12,
            orientation_mod: 2,
        },
        OrbitDef {
            name: "corners".to_owned(),
            size: 8,
            orientation_mod: 3,
        },
    ];
    let moves = vec![
        Move {
            name: "F".to_owned(),
            r#move: PuzzleState::from_orbit_states([
                9, 0, 2, 3, 1, 5, 6, 7, 8, 4, 10, 11, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 6, 0, 2,
                1, 4, 5, 3, 7, 2, 1, 0, 2, 0, 0, 1, 0,
            ]),
        },
        Move {
            name: "B".to_owned(),
            r#move: PuzzleState::from_orbit_states([
                0, 1, 5, 3, 4, 6, 10, 7, 8, 9, 2, 11, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0, 1, 4,
                3, 7, 2, 6, 5, 0, 0, 1, 0, 2, 2, 0, 1,
            ]),
        },
        Move {
            name: "D".to_owned(),
            r#move: PuzzleState::from_orbit_states([
                0, 8, 2, 1, 4, 3, 6, 7, 5, 9, 10, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 2,
                7, 1, 5, 6, 4, 0, 0, 0, 0, 0, 0, 0, 0,
            ]),
        },
        Move {
            name: "U".to_owned(),
            r#move: PuzzleState::from_orbit_states([
                0, 1, 2, 3, 4, 5, 6, 10, 8, 7, 11, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 1, 5,
                3, 4, 6, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0,
            ]),
        },
        Move {
            name: "L".to_owned(),
            r#move: PuzzleState::from_orbit_states([
                0, 1, 2, 3, 11, 5, 8, 7, 4, 9, 10, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2,
                6, 4, 7, 5, 3, 0, 0, 0, 1, 0, 1, 2, 2,
            ]),
        },
        Move {
            name: "R".to_owned(),
            r#move: PuzzleState::from_orbit_states([
                3, 1, 7, 2, 4, 5, 6, 0, 8, 9, 10, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 4, 0,
                3, 2, 5, 6, 7, 1, 2, 2, 0, 1, 0, 0, 0,
            ]),
        },
    ];
    PuzzleDef {
        name: "cube3".to_owned(),
        orbit_defs,
        moves,
    }
});
