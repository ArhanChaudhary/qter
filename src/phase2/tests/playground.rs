use itertools::Itertools;
use log::info;
use phase2::{
    make_guard,
    pruning::{
        OrbitPruningTables, OrbitPruningTablesGenerateMeta, PruningTables, StorageBackendTy,
        TableTy,
    },
    puzzle::{PuzzleDef, SortedCycleType, cube3::Cube3},
    solver::{CycleTypeSolver, SearchStrategy},
};
use puzzle_geometry::ksolve::KPUZZLE_3X3;

#[test_log::test]
fn playground() {
    make_guard!(guard);
    let cube3_def = PuzzleDef::<Cube3>::new(&KPUZZLE_3X3, guard).unwrap();
    let sorted_cycle_type = SortedCycleType::new(
        &[vec![(3, false)], vec![]],
        cube3_def.sorted_orbit_defs_ref(),
    )
    .unwrap();
    let generate_meta = OrbitPruningTablesGenerateMeta::new_with_table_types(
        &cube3_def,
        vec![
            // TableTy::Exact(StorageBackendTy::Uncompressed),
            TableTy::Zero,
            TableTy::Zero,
        ],
        88_179_840,
        cube3_def.id(),
    )
    .unwrap();
    let pruning_tables =
        OrbitPruningTables::try_generate_all(sorted_cycle_type, generate_meta).unwrap();
    let solver: CycleTypeSolver<Cube3, _> =
        CycleTypeSolver::new(cube3_def, pruning_tables, SearchStrategy::AllSolutions);

    let mut solutions = solver.solve::<[Cube3; 21]>().unwrap();
    while solutions.next().is_some() {
        info!(
            "{:<2}",
            solutions
                .current_solution()
                .unwrap()
                .iter()
                .map(|move_| move_.name())
                .format(" ")
        );
    }

    panic!();
}
