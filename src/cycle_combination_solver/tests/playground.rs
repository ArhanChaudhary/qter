#![allow(unused_imports)]

use cycle_combination_solver::{
    make_guard,
    pruning::{
        OrbitPruningTables, OrbitPruningTablesGenerateMeta, PruningTables, StorageBackendTy,
        TableTy,
    },
    puzzle::{PuzzleDef, SortedCycleStructure, cube3::Cube3},
    solver::{CycleStructureSolver, SearchStrategy},
};
use itertools::Itertools;
use log::info;
use puzzle_geometry::ksolve::KPUZZLE_3X3;

#[test_log::test]
fn playground() {}
