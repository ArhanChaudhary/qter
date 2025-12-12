#![allow(unused_imports, unused_variables)]

use cycle_combination_solver::{
    make_guard,
    pruning::{
        OrbitPruningTables, OrbitPruningTablesGenerateMeta, PruningTables, StorageBackendTy,
        TableTy,
    },
    puzzle::{
        PuzzleDef, PuzzleState, SortedCycleStructure, apply_moves, cube3::Cube3,
        slice_puzzle::HeapPuzzle,
    },
    solver::{CycleStructureSolver, SearchStrategy},
};
use itertools::Itertools;
use log::info;
use puzzle_geometry::ksolve::{KPUZZLE_3X3, KSolve};

#[test_log::test]
fn playground() {
    make_guard!(guard);
    let cube3_def = PuzzleDef::<HeapPuzzle>::new(&KPUZZLE_3X3, guard).unwrap();
    println!("{:?}", cube3_def);
    let solved = cube3_def.new_solved_state();
    let a = apply_moves(&cube3_def, &solved, "U D R U D", 1);
    println!(
        "{:?}",
        a.sorted_cycle_structure(
            cube3_def.sorted_orbit_defs_ref(),
            &mut HeapPuzzle::new_aux_mem(cube3_def.sorted_orbit_defs_ref())
        )
    );
    panic!();
    // let mut result_1 = solved.clone();
    // let mut result_2 = result_1.clone();
    // let move_1 = cube3_def.find_move("B2").unwrap();
    // let move_2 = cube3_def.find_move("F2").unwrap();
    // result_1.replace_compose(
    //     &result_2,
    //     move_1.puzzle_state(),
    //     cube3_def.sorted_orbit_defs_ref(),
    // );
    // result_2.replace_compose(
    //     &result_1,
    //     move_2.puzzle_state(),
    //     cube3_def.sorted_orbit_defs_ref(),
    // );
    // assert_eq!(a, solved);
}
