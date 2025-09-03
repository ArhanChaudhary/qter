#![allow(unused_imports)]

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
fn playground() {}
