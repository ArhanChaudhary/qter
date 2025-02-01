

// use super::puzzle::{
//     HeapStorage, Move, OrbitDef, PuzzleDef, PuzzleState, PuzzleStateCore, StackStorage, Storage,
// };

// trait PuzzleDefCore<S: Storage> {
//     fn def() -> PuzzleDef<S>;
//     fn orbit_defs() -> Vec<OrbitDef>;
//     fn moves() -> Vec<(String, &'static [u8])>;
// }

// struct Cube3Def;

// impl Cube3Def {
//     fn orbit_defs() -> Vec<OrbitDef> {
//         vec![
//             OrbitDef {
//                 name: "edges".to_owned(),
//                 size: 12,
//                 orientation_mod: 2,
//             },
//             OrbitDef {
//                 name: "corners".to_owned(),
//                 size: 8,
//                 orientation_mod: 3,
//             },
//         ]
//     }
// }

// impl PuzzleDefCore<StackStorage<40>> for Cube3Def {
//     fn orbit_defs() -> Vec<OrbitDef> {
//         Cube3Def::orbit_defs()
//     }

//     fn moves() -> Vec<(String, &'static [u8])> {
//         vec![
//             (
//                 "F".to_owned(),
//                 &[
//                     9, 0, 2, 3, 1, 5, 6, 7, 8, 4, 10, 11, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 6, 0,
//                     2, 1, 4, 5, 3, 7, 2, 1, 0, 2, 0, 0, 1, 0,
//                 ],
//             ),
//             (
//                 "B".to_owned(),
//                 &[
//                     0, 1, 5, 3, 4, 6, 10, 7, 8, 9, 2, 11, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0, 1,
//                     4, 3, 7, 2, 6, 5, 0, 0, 1, 0, 2, 2, 0, 1,
//                 ],
//             ),
//             (
//                 "D".to_owned(),
//                 &[
//                     0, 8, 2, 1, 4, 3, 6, 7, 5, 9, 10, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,
//                     2, 7, 1, 5, 6, 4, 0, 0, 0, 0, 0, 0, 0, 0,
//                 ],
//             ),
//             (
//                 "U".to_owned(),
//                 &[
//                     0, 1, 2, 3, 4, 5, 6, 10, 8, 7, 11, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 1,
//                     5, 3, 4, 6, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0,
//                 ],
//             ),
//             (
//                 "L".to_owned(),
//                 &[
//                     0, 1, 2, 3, 11, 5, 8, 7, 4, 9, 10, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
//                     2, 6, 4, 7, 5, 3, 0, 0, 0, 1, 0, 1, 2, 2,
//                 ],
//             ),
//             (
//                 "R".to_owned(),
//                 &[
//                     3, 1, 7, 2, 4, 5, 6, 0, 8, 9, 10, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 4,
//                     0, 3, 2, 5, 6, 7, 1, 2, 2, 0, 1, 0, 0, 0,
//                 ],
//             ),
//         ]
//     }
// }

// impl PuzzleDefCore<HeapStorage> for Cube3Def {
//     fn def() -> PuzzleDef<HeapStorage> {
//         let orbit_defs = vec![
//             OrbitDef {
//                 name: "edges".to_owned(),
//                 size: 12,
//                 orientation_mod: 2,
//             },
//             OrbitDef {
//                 name: "corners".to_owned(),
//                 size: 8,
//                 orientation_mod: 3,
//             },
//         ];
//         let moves = vec![
//             Move {
//                 name: "F".to_owned(),
//                 delta: PuzzleState::from_orbit_states(
//                     vec![
//                         9, 0, 2, 3, 1, 5, 6, 7, 8, 4, 10, 11, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0,
//                         6, 0, 2, 1, 4, 5, 3, 7, 2, 1, 0, 2, 0, 0, 1, 0,
//                     ]
//                     .into_boxed_slice(),
//                 ),
//             },
//             Move {
//                 name: "B".to_owned(),
//                 delta: PuzzleState::from_orbit_states(
//                     vec![
//                         0, 1, 5, 3, 4, 6, 10, 7, 8, 9, 2, 11, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0,
//                         0, 1, 4, 3, 7, 2, 6, 5, 0, 0, 1, 0, 2, 2, 0, 1,
//                     ]
//                     .into_boxed_slice(),
//                 ),
//             },
//             Move {
//                 name: "D".to_owned(),
//                 delta: PuzzleState::from_orbit_states(
//                     vec![
//                         0, 8, 2, 1, 4, 3, 6, 7, 5, 9, 10, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//                         0, 3, 2, 7, 1, 5, 6, 4, 0, 0, 0, 0, 0, 0, 0, 0,
//                     ]
//                     .into_boxed_slice(),
//                 ),
//             },
//             Move {
//                 name: "U".to_owned(),
//                 delta: PuzzleState::from_orbit_states(
//                     vec![
//                         0, 1, 2, 3, 4, 5, 6, 10, 8, 7, 11, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//                         2, 1, 5, 3, 4, 6, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0,
//                     ]
//                     .into_boxed_slice(),
//                 ),
//             },
//             Move {
//                 name: "L".to_owned(),
//                 delta: PuzzleState::from_orbit_states(
//                     vec![
//                         0, 1, 2, 3, 11, 5, 8, 7, 4, 9, 10, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//                         0, 1, 2, 6, 4, 7, 5, 3, 0, 0, 0, 1, 0, 1, 2, 2,
//                     ]
//                     .into_boxed_slice(),
//                 ),
//             },
//             Move {
//                 name: "R".to_owned(),
//                 delta: PuzzleState::from_orbit_states(
//                     vec![
//                         3, 1, 7, 2, 4, 5, 6, 0, 8, 9, 10, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//                         1, 4, 0, 3, 2, 5, 6, 7, 1, 2, 2, 0, 1, 0, 0, 0,
//                     ]
//                     .into_boxed_slice(),
//                 ),
//             },
//         ];
//         PuzzleDef {
//             name: "cube3".to_owned(),
//             orbit_defs,
//             moves,
//         }
//     }
// }
