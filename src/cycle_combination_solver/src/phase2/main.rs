// use cycle_combination_solver::phase2::puzzle::{PuzzleDef, PuzzleState, StackCube3};
// use puzzle_geometry::ksolve::KPUZZLE_3X3;

// use cycle_combination_solver::phase2::{puzzle::induces_sorted_cycle_type_slice, solver::CycleTypeSolver};
use itertools::{repeat_n, Itertools};
use puzzle_geometry::ksolve::KPUZZLE_3X3;

fn main() {
    // use super::*;

    // use cube::CubeState;
    // use pruning::PruningTables;
    // use solver::CycleTypeSolver;
    // use std::time::Instant;

    // #[test]
    // fn test_cycle_type() {
    //     let cycle_type = CycleType {
    //         corner_partition: vec![(3, true), (5, true)],
    //         edge_partition: vec![(2, true), (2, true)],
    //     };

    //     let mut tag = "corners".to_string();
    //     for &(corner, orient) in cycle_type.corner_partition.iter() {
    //         tag.push_str(&format!("{}{}", corner, if orient { "o" } else { "n" }));
    //     }
    //     let pruning_tables = PruningTables::from(&tag, &cycle_type);
    //     let now = Instant::now();
    //     let mut solver = CycleTypeSolver::new(CubeState::default(), &pruning_tables, cycle_type);
    //     let solution = solver.solve();
    //     let elapsed = now.elapsed();
    //     println!("{}", solution);
    //     println!("Found phase 2 solution in {:.2?}", elapsed);
    // }
    // let mut solved = CycleTypeSolver::new(
    //     &*KPUZZLE_3X3,
    //     vec![
    //         vec![(3, true), (5, true)],
    //         vec![(2, true), (2, true)],
    //     ],
    //     BloomFilterTable::new(),
    // );

    // let mut depth = 0;
    // let cubies = 8;
    // let orientation_count = 3;
    // for (cp_index, cp) in (0..cubies).permutations(cubies as usize).enumerate() {
    //     for (co_index, co) in repeat_n(0..orientation_count, cubies as usize)
    //         .multi_cartesian_product()
    //         // TODO more efficient way than filtering
    //         .filter(|p| p.iter().sum::<i8>().rem_euclid(orientation_count) == 0)
    //         .enumerate()
    //     {
    //         if induces_sorted_cycle_type_slice(
    //             orbit_states,
    //             sorted_cycle_type,
    //             multi_bv,
    //             sorted_orbit_defs,
    //         ) {
    //             depth += 1;
    //         }
    //     }
    // }
    // println!("{}", depth);
}
