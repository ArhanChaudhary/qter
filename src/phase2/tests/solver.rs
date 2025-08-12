use itertools::Itertools;
use log::info;
use phase2::{
    make_guard,
    pruning::{
        OrbitPruningTables, OrbitPruningTablesGenerateMeta, PruningTables, StorageBackendTy,
        TableTy, ZeroTable,
    },
    puzzle::{
        MultiBvInterface, PuzzleDef, PuzzleState, SortedCycleType, cube3::Cube3,
        slice_puzzle::HeapPuzzle,
    },
    solver::{CycleTypeSolver, SearchStrategy},
};
use puzzle_geometry::ksolve::{KPUZZLE_3X3, KPUZZLE_4X4};

#[test_log::test]
fn test_identity_cycle_type() {
    make_guard!(guard);
    let (cube3_def, id) = PuzzleDef::<Cube3>::new(&KPUZZLE_3X3, guard).unwrap();
    let identity_cycle_type =
        SortedCycleType::new(vec![vec![], vec![]], cube3_def.sorted_orbit_defs_ref()).unwrap();

    let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
        &cube3_def,
        identity_cycle_type.clone(),
        ZeroTable::try_generate_all(()).unwrap(),
        SearchStrategy::AllSolutions,
    );
    let solutions = solver.solve::<[Cube3; 21]>().collect_vec();
    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].len(), 0);

    let pruning_tables = OrbitPruningTables::try_generate_all(
        OrbitPruningTablesGenerateMeta::new_with_table_types(
            &cube3_def,
            &identity_cycle_type,
            vec![
                (TableTy::Exact, StorageBackendTy::Zero),
                (TableTy::Exact, StorageBackendTy::Zero),
            ],
            0,
            id,
        )
        .unwrap(),
    )
    .unwrap();
    let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
        &cube3_def,
        identity_cycle_type,
        pruning_tables,
        SearchStrategy::AllSolutions,
    );
    let solutions = solver.solve::<[Cube3; 21]>().collect_vec();
    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].len(), 0);
}

#[test_log::test]
fn test_single_quarter_turn() {
    make_guard!(guard);
    let cube3_def = PuzzleDef::<Cube3>::new(&KPUZZLE_3X3, guard).unwrap().0;
    let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
        &cube3_def,
        SortedCycleType::new(
            vec![vec![(4, false)], vec![(4, false)]],
            cube3_def.sorted_orbit_defs_ref(),
        )
        .unwrap(),
        ZeroTable::try_generate_all(()).unwrap(),
        SearchStrategy::AllSolutions,
    );
    let solutions = solver.solve::<[Cube3; 21]>().collect_vec();
    assert_eq!(solutions.len(), 12);
    assert!(solutions.iter().all(|solution| solution.len() == 1));
}

#[test_log::test]
fn test_single_half_turn() {
    make_guard!(guard);
    let cube3_def = PuzzleDef::<Cube3>::new(&KPUZZLE_3X3, guard).unwrap().0;
    let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
        &cube3_def,
        SortedCycleType::new(
            vec![vec![(2, false), (2, false)], vec![(2, false), (2, false)]],
            cube3_def.sorted_orbit_defs_ref(),
        )
        .unwrap(),
        ZeroTable::try_generate_all(()).unwrap(),
        SearchStrategy::AllSolutions,
    );
    let solutions = solver.solve::<[Cube3; 21]>().collect_vec();
    assert_eq!(solutions.len(), 6);
    assert!(solutions.iter().all(|solution| solution.len() == 1));
}

#[test_log::test]
fn test_optimal_subgroup_cycle() {
    make_guard!(guard);
    let cube3_def =
        PuzzleDef::<Cube3>::new(&KPUZZLE_3X3.clone().with_moves(&["F", "R", "U"]), guard)
            .unwrap()
            .0;
    let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
        &cube3_def,
        SortedCycleType::new(
            vec![vec![(3, false), (4, false)], vec![(1, true), (8, true)]],
            cube3_def.sorted_orbit_defs_ref(),
        )
        .unwrap(),
        ZeroTable::try_generate_all(()).unwrap(),
        SearchStrategy::AllSolutions,
    );
    let solutions = solver.solve::<[Cube3; 21]>().collect_vec();
    // for solution in &solutions {
    //     for move_ in solution {
    //         print!("{} ", &move_.name);
    //     }
    //     println!();
    // }
    assert_eq!(solutions.len(), 22); // TODO: should be 24
    assert!(solutions.iter().all(|solution| solution.len() == 4));
}

#[test_log::test]
fn test_control_optimal_cycle() {
    return;
    make_guard!(guard);
    let (cube3_def, id) = PuzzleDef::<Cube3>::new(&KPUZZLE_3X3, guard).unwrap();
    let sorted_cycle_type = SortedCycleType::new(
        vec![vec![(1, true), (5, true)], vec![(1, true), (1, true)]],
        cube3_def.sorted_orbit_defs_ref(),
    )
    .unwrap();
    let generate_meta = OrbitPruningTablesGenerateMeta::new_with_table_types(
        &cube3_def,
        &sorted_cycle_type,
        vec![
            (TableTy::Exact, StorageBackendTy::Uncompressed),
            (TableTy::Zero, StorageBackendTy::Zero),
        ],
        88_179_840,
        id,
    )
    .unwrap();
    let pruning_tables = OrbitPruningTables::try_generate_all(generate_meta).unwrap();
    let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
        &cube3_def,
        sorted_cycle_type,
        pruning_tables,
        SearchStrategy::AllSolutions,
    );

    let solutions = solver.solve::<[Cube3; 21]>().collect_vec();
    for solution in &solutions {
        for move_ in solution {
            info!("{} ", &move_.name);
        }
        info!("\n");
    }
    assert_eq!(solutions.len(), 260); // TODO: should be 480
    assert!(solutions.iter().all(|solution| solution.len() == 5));
}

#[allow(dead_code)]
struct OptimalCycleTypeTest {
    moves_str: &'static str,
    expected_partial_count: usize,
    expected_count: usize,
}

#[test_log::test]
fn test_many_optimal_cycles() {
    make_guard!(guard);
    let cube3_def = PuzzleDef::<HeapPuzzle>::new(&KPUZZLE_3X3, guard).unwrap().0;

    // let sorted_cycle
    let mut solver: CycleTypeSolver<HeapPuzzle, _> = CycleTypeSolver::new(
        &cube3_def,
        SortedCycleType::new(vec![vec![], vec![]], cube3_def.sorted_orbit_defs_ref()).unwrap(),
        ZeroTable::try_generate_all(()).unwrap(),
        SearchStrategy::AllSolutions,
    );

    // Test cases taken from Michael Gottlieb's order table
    // https://mzrg.com/rubik/orders.shtml
    let optimal_cycle_type_tests = [
        OptimalCycleTypeTest {
            moves_str: "U2 R2 U2 R2",
            expected_partial_count: 24,
            expected_count: 24,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U R'",
            expected_partial_count: 188,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "U2 R2",
            expected_partial_count: 24,
            expected_count: 24,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U' F",
            expected_partial_count: 360,
            expected_count: 384,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2 D",
            expected_partial_count: 92,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "U R F2",
            expected_partial_count: 140,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U' F2",
            expected_partial_count: 368,
            expected_count: 384,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2 B2",
            expected_partial_count: 142,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2 U R2",
            expected_partial_count: 96,
            expected_count: 96,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U' L2",
            expected_partial_count: 372,
            expected_count: 384,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2 F R2",
            expected_partial_count: 472,
            expected_count: 480,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2 D2",
            expected_partial_count: 282,
            expected_count: 432,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U' L",
            expected_partial_count: 368,
            expected_count: 384,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2",
            expected_partial_count: 96,
            expected_count: 96,
        },
        OptimalCycleTypeTest {
            moves_str: "U R F' D'",
            expected_partial_count: 212,
            expected_count: 384,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2 D' R2",
            expected_partial_count: 96,
            expected_count: 96,
        },
        OptimalCycleTypeTest {
            moves_str: "U R B2",
            expected_partial_count: 140,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U2 L",
            expected_partial_count: 744,
            expected_count: 768,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2 U2 R'",
            expected_partial_count: 188,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "U R B' D'",
            expected_partial_count: 212,
            expected_count: 384,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U L",
            expected_partial_count: 282,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U2 B",
            expected_partial_count: 182,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "U R B' L2",
            expected_partial_count: 804,
            expected_count: 1152,
        },
        OptimalCycleTypeTest {
            moves_str: "U R B'",
            expected_partial_count: 138,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "U R'",
            expected_partial_count: 48,
            expected_count: 48,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U F'",
            expected_partial_count: 368,
            expected_count: 384,
        },
        OptimalCycleTypeTest {
            moves_str: "U R B' L",
            expected_partial_count: 180,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "U R B",
            expected_partial_count: 46,
            expected_count: 48,
        },
        OptimalCycleTypeTest {
            moves_str: "U R F",
            expected_partial_count: 46,
            expected_count: 48,
        },
        OptimalCycleTypeTest {
            moves_str: "U R D",
            expected_partial_count: 90,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2 F L2",
            expected_partial_count: 184,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "U R",
            expected_partial_count: 48,
            expected_count: 48,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U L2",
            expected_partial_count: 376,
            expected_count: 384,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U F2",
            expected_partial_count: 372,
            expected_count: 384,
        },
        OptimalCycleTypeTest {
            moves_str: "U R F' L",
            expected_partial_count: 180,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "U R' U F'",
            expected_partial_count: 184,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "U R D2",
            expected_partial_count: 184,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "U R D'",
            expected_partial_count: 180,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "U R B2 F2",
            expected_partial_count: 228,
            expected_count: 384,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2 U F'",
            expected_partial_count: 372,
            expected_count: 384,
        },
        OptimalCycleTypeTest {
            moves_str: "U R F2 L2",
            expected_partial_count: 2432,
            expected_count: 3456,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2 B L'",
            expected_partial_count: 182,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "U R F L",
            expected_partial_count: 90,
            expected_count: 96,
        },
        OptimalCycleTypeTest {
            moves_str: "U R D L",
            expected_partial_count: 46,
            expected_count: 48,
        },
        OptimalCycleTypeTest {
            moves_str: "U R F'",
            expected_partial_count: 138,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "U R U2 F",
            expected_partial_count: 182,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "U R2 F L'",
            expected_partial_count: 182,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "U R B2 F'",
            expected_partial_count: 220,
            expected_count: 384,
        },
    ];

    let solved = cube3_def.new_solved_state();
    let mut multi_bv = HeapPuzzle::new_multi_bv(cube3_def.sorted_orbit_defs_ref());

    for optimal_cycle_test in optimal_cycle_type_tests {
        let mut result_1 = solved.clone();
        let mut result_2 = solved.clone();
        let mut move_count = 0;
        for name in optimal_cycle_test.moves_str.split_whitespace() {
            let move_ = cube3_def.find_move(name).unwrap();
            result_2.replace_compose(
                &result_1,
                &move_.puzzle_state,
                cube3_def.sorted_orbit_defs_ref(),
            );
            std::mem::swap(&mut result_1, &mut result_2);
            move_count += 1;
        }

        let cycle_type =
            result_1.cycle_type(cube3_def.sorted_orbit_defs_ref(), multi_bv.reusable_ref());
        solver.set_sorted_cycle_type(cycle_type);

        let solutions = solver.solve::<Vec<_>>().collect_vec();
        assert_eq!(solutions.len(), optimal_cycle_test.expected_partial_count);
        assert!(
            solutions
                .iter()
                .all(|solution| solution.len() == move_count)
        );
    }
}

#[test_log::test]
fn test_big_cube_optimal_cycle() {
    make_guard!(guard);
    let cube4_def = PuzzleDef::<HeapPuzzle>::new(&KPUZZLE_4X4, guard).unwrap().0;

    let mut solver: CycleTypeSolver<HeapPuzzle, _> = CycleTypeSolver::new(
        &cube4_def,
        SortedCycleType::new(
            vec![vec![], vec![], vec![]],
            cube4_def.sorted_orbit_defs_ref(),
        )
        .unwrap(),
        ZeroTable::try_generate_all(()).unwrap(),
        SearchStrategy::AllSolutions,
    );

    // Test cases taken from Michael Gottlieb's order table
    // https://mzrg.com/rubik/orders.shtml
    let mut optimal_cycle_type_tests = [
        OptimalCycleTypeTest {
            moves_str: "R2",
            expected_partial_count: 6,
            expected_count: 6,
        },
        OptimalCycleTypeTest {
            moves_str: "r2 u2",
            expected_partial_count: 24,
            expected_count: 24,
        },
        OptimalCycleTypeTest {
            moves_str: "R",
            expected_partial_count: 12,
            expected_count: 12,
        },
        OptimalCycleTypeTest {
            moves_str: "R2 U2",
            expected_partial_count: 24,
            expected_count: 24,
        },
        OptimalCycleTypeTest {
            moves_str: "r u' f2",
            expected_partial_count: 288,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "r u'",
            expected_partial_count: 48,
            expected_count: 48,
        },
        OptimalCycleTypeTest {
            moves_str: "r u",
            expected_partial_count: 48,
            expected_count: 48,
        },
        OptimalCycleTypeTest {
            moves_str: "R L' 2U",
            expected_partial_count: 184,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R 2U",
            expected_partial_count: 192,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "r l2 u",
            expected_partial_count: 192,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "2R 2U",
            expected_partial_count: 96,
            expected_count: 96,
        },
        OptimalCycleTypeTest {
            moves_str: "R U2",
            expected_partial_count: 96,
            expected_count: 96,
        },
        OptimalCycleTypeTest {
            moves_str: "R L 2U",
            expected_partial_count: 184,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R U'",
            expected_partial_count: 48,
            expected_count: 48,
        },
        OptimalCycleTypeTest {
            moves_str: "r 2U",
            expected_partial_count: 192,
            expected_count: 192,
        },
        OptimalCycleTypeTest {
            moves_str: "F U R",
            expected_partial_count: 46,
            expected_count: 48,
        },
        OptimalCycleTypeTest {
            moves_str: "R' 2U 2F'",
            expected_partial_count: 284,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R L U",
            expected_partial_count: 90,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "R U",
            expected_partial_count: 48,
            expected_count: 48,
        },
        OptimalCycleTypeTest {
            moves_str: "R l' 2U",
            expected_partial_count: 188,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R u' 2F'",
            expected_partial_count: 568,
            expected_count: 576,
        },
        OptimalCycleTypeTest {
            moves_str: "r' 2U 2F",
            expected_partial_count: 144,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "R L2 U",
            expected_partial_count: 184,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R L' U",
            expected_partial_count: 180,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R u",
            expected_partial_count: 96,
            expected_count: 96,
        },
        OptimalCycleTypeTest {
            moves_str: "R u 2F'",
            expected_partial_count: 568,
            expected_count: 576,
        },
        OptimalCycleTypeTest {
            moves_str: "r 2U' 2F'",
            expected_partial_count: 144,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "R u f",
            expected_partial_count: 142,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "r' 2U 2F'",
            expected_partial_count: 288,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R u' 2F",
            expected_partial_count: 284,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "F U R'",
            expected_partial_count: 138,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "R U f'",
            expected_partial_count: 140,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "R u' 2L",
            expected_partial_count: 188,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R u' 2L'",
            expected_partial_count: 188,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R u'",
            expected_partial_count: 96,
            expected_count: 96,
        },
        OptimalCycleTypeTest {
            moves_str: "R' U' f",
            expected_partial_count: 140,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "R2 u f'",
            expected_partial_count: 288,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R U' f'",
            expected_partial_count: 280,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R U l",
            expected_partial_count: 184,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "r U' 2L'",
            expected_partial_count: 188,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R2 u 2F",
            expected_partial_count: 576,
            expected_count: 576,
        },
        OptimalCycleTypeTest {
            moves_str: "R u 2L",
            expected_partial_count: 188,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R l u'",
            expected_partial_count: 188,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R2 u' f'",
            expected_partial_count: 144,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "R l' u'",
            expected_partial_count: 188,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R' U2 f",
            expected_partial_count: 284,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R U l'",
            expected_partial_count: 368,
            expected_count: 576,
        },
        OptimalCycleTypeTest {
            moves_str: "r' u' 2F2",
            expected_partial_count: 144,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "r u' 2F2",
            expected_partial_count: 288,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R u' f'",
            expected_partial_count: 142,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "R u 2L'",
            expected_partial_count: 188,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R l u",
            expected_partial_count: 188,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "r' u' 2F",
            expected_partial_count: 144,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "R2 u f",
            expected_partial_count: 144,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "r u 2L2",
            expected_partial_count: 192,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R u 2F2",
            expected_partial_count: 284,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "r u 2L",
            expected_partial_count: 192,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R2 l u'",
            expected_partial_count: 192,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R2 l u",
            expected_partial_count: 192,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R l2 u'",
            expected_partial_count: 188,
            expected_count: 288,
        },
        OptimalCycleTypeTest {
            moves_str: "R' u f",
            expected_partial_count: 142,
            expected_count: 144,
        },
        OptimalCycleTypeTest {
            moves_str: "R2 r u'",
            expected_partial_count: 572,
            expected_count: 864,
        },
        OptimalCycleTypeTest {
            moves_str: "R2 r u",
            expected_partial_count: 572,
            expected_count: 864,
        },
    ];

    fastrand::shuffle(&mut optimal_cycle_type_tests);
    // only do 5 because this is slow
    let optimal_cycle_type_tests = &optimal_cycle_type_tests[0..5];

    let solved = cube4_def.new_solved_state();
    let mut multi_bv = HeapPuzzle::new_multi_bv(cube4_def.sorted_orbit_defs_ref());

    for optimal_cycle_test in optimal_cycle_type_tests {
        let mut result_1 = solved.clone();
        let mut result_2 = solved.clone();
        let mut move_count = 0;
        for name in optimal_cycle_test.moves_str.split_whitespace() {
            let move_ = cube4_def.find_move(name).unwrap();
            result_2.replace_compose(
                &result_1,
                &move_.puzzle_state,
                cube4_def.sorted_orbit_defs_ref(),
            );
            std::mem::swap(&mut result_1, &mut result_2);
            move_count += 1;
        }

        let cycle_type =
            result_1.cycle_type(cube4_def.sorted_orbit_defs_ref(), multi_bv.reusable_ref());
        solver.set_sorted_cycle_type(cycle_type);

        let solutions = solver.solve::<Vec<_>>().collect_vec();
        assert_eq!(solutions.len(), optimal_cycle_test.expected_partial_count);
        assert!(
            solutions
                .iter()
                .all(|solution| solution.len() == move_count)
        );
    }
}
