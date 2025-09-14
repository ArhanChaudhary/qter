#![allow(unused_imports)]

use cycle_combination_solver::{
    make_guard,
    pruning::{
        OrbitPruningTables, OrbitPruningTablesGenerateMeta, PruningTables, StorageBackendTy,
        TableTy,
    },
    puzzle::{PuzzleDef, SortedCycleType, cube3::Cube3},
    solver::{CycleTypeSolver, SearchStrategy},
};
use itertools::Itertools;
use log::info;
use puzzle_geometry::ksolve::KPUZZLE_3X3;

#[test_log::test]
fn playground() {}
