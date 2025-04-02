use super::{
    canonical_fsm::{CanonicalFSM, CanonicalFSMState},
    pruning::PruningTables,
    puzzle::{Move, MultiBvInterface, OrientedPartition, PuzzleDef, PuzzleState},
    puzzle_state_history::{PuzzleStateHistory, PuzzleStateHistoryInterface},
};

pub struct CycleTypeSolver<'a, P: PuzzleState, T: PruningTables<P>> {
    puzzle_def: &'a PuzzleDef<P>,
    canonical_fsm: CanonicalFSM<P>,
    sorted_cycle_type: Vec<OrientedPartition>,
    pruning_tables: T,
}

struct CycleTypeSolverMutable<P: PuzzleState, H: PuzzleStateHistoryInterface<P>> {
    puzzle_state_history: PuzzleStateHistory<P, H>,
    multi_bv: P::MultiBv,
    // TODO: list of usize until the very end at fn return
    solutions: Vec<Box<[Move<P>]>>,
    first_move_class_index: usize,
}

impl<'a, P: PuzzleState, T: PruningTables<P>> CycleTypeSolver<'a, P, T> {
    pub fn new(
        puzzle_def: &'a PuzzleDef<P>,
        sorted_cycle_type: Vec<OrientedPartition>,
        pruning_tables: T,
    ) -> Self {
        let canonical_fsm = puzzle_def.into();
        Self {
            puzzle_def,
            canonical_fsm,
            sorted_cycle_type,
            pruning_tables,
        }
    }

    pub fn set_sorted_cycle_type(&mut self, sorted_cycle_type: Vec<OrientedPartition>) {
        self.sorted_cycle_type = sorted_cycle_type;
    }

    fn search_for_solution<H: PuzzleStateHistoryInterface<P>>(
        &self,
        mutable: &mut CycleTypeSolverMutable<P, H>,
        current_fsm_state: CanonicalFSMState,
        entry_index: usize,
        root: bool,
        mut togo: u8,
    ) {
        // SAFETY: This function calls `pop_stack` for every `push_stack` call.
        // Therefore, the `pop_stack` cannot be called more than `push_stack`.
        let last_puzzle_state = unsafe { mutable.puzzle_state_history.last_state_unchecked() };
        let est_remaining_cost = self.pruning_tables.permissible_heuristic(last_puzzle_state);

        if est_remaining_cost > togo {
            // TODO: what the heck does this do
            // https://github.com/cubing/twsearch/commit/a86177ac2bd462bb9d7d91af743e883449fbfb6b
            return;
        }

        if togo == 0 {
            if last_puzzle_state.induces_sorted_cycle_type(
                &self.sorted_cycle_type,
                &self.puzzle_def.sorted_orbit_defs,
                mutable.multi_bv.reusable_ref(),
            ) {
                mutable.solutions.push(
                    mutable
                        .puzzle_state_history
                        .create_move_history(self.puzzle_def),
                );
            }
            return;
        }

        // TODO: this doesn't cover every symmetric sequence
        let mut next_entry_index = entry_index + 1;
        // SAFETY: `entry_index` starts at zero in the initial call, and
        // `B::initialize` guarantees that the first entry is bound. For every
        // recursive call, the puzzle history stack is pushed and `entry_index`
        // can only be incremented by 1. Therefore, `entry_index` is always
        // less than the number of entries in the puzzle state history and
        // always bound
        let start = unsafe {
            mutable
                .puzzle_state_history
                .move_index_unchecked(entry_index)
        };
        togo -= 1;
        for (move_index, move_) in self.puzzle_def.moves.iter().enumerate().skip(start) {
            let move_class_index = move_.move_class_index;
            // branches should have high predictability
            if root {
                mutable.first_move_class_index = move_class_index;
            } else if togo == 0 && move_class_index == mutable.first_move_class_index {
                // we don't have to set `next_entry_index = 0` here because
                // `togo` is already zero
                continue;
            }

            let next_fsm_state = self
                .canonical_fsm
                .next_state(current_fsm_state, move_class_index);
            if next_fsm_state.is_none() {
                next_entry_index = 0;
                continue;
            };

            // SAFETY:
            // 1) `pop_stack` is called for every `push_stack` call, so
            //    pop_stack cannot be called more than push_stack.
            // 2) `resize_if_needed` is appropriately called in `solve` before
            //    every call to this function.
            // 3) `move_index` is defined to be bound.
            unsafe {
                mutable
                    .puzzle_state_history
                    .push_stack_unchecked(move_index, self.puzzle_def);
            }
            // TODO: Actual IDA* takes the min of this bound and uses it
            self.search_for_solution(mutable, next_fsm_state, next_entry_index, false, togo);
            mutable.puzzle_state_history.pop_stack();
            next_entry_index = 0;

            // TODO: BPMX optimization
            // if next_est_goal_cost.saturating_sub(1) > est_goal_cost {
            //     est_goal_cost = next_est_goal_cost.saturating_sub(1);
            //     if est_goal_cost > cost_bound {
            //         return est_goal_cost;
            //     }
            // }
        }
    }

    pub fn solve<H: PuzzleStateHistoryInterface<P>>(&self) -> Vec<Box<[Move<P>]>> {
        let mut mutable: CycleTypeSolverMutable<P, H> = CycleTypeSolverMutable {
            puzzle_state_history: self.puzzle_def.into(),
            multi_bv: P::new_multi_bv(&self.puzzle_def.sorted_orbit_defs),
            solutions: vec![],
            first_move_class_index: usize::default(),
        };
        let mut depth = self.pruning_tables.permissible_heuristic(
            // SAFETY: `H::initialize` when puzzle_state_history is created
            // guarantees that the first entry is bound
            unsafe { mutable.puzzle_state_history.last_state_unchecked() },
        );
        mutable
            .puzzle_state_history
            .resize_if_needed(depth as usize + 1);
        while mutable.solutions.is_empty() {
            eprintln!("Searching depth {}...", depth);
            self.search_for_solution(&mut mutable, CanonicalFSMState::default(), 0, true, depth);
            mutable
                .puzzle_state_history
                .resize_if_needed(depth as usize + 1);
            depth += 1;
        }

        mutable.solutions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::{
        pruning::{
            NullMeta, OrbitPruningTableTy, OrbitPruningTables, OrbitPruningTablesGenerateMeta,
            StorageBackendTy, ZeroTable,
        },
        puzzle::{HeapPuzzle, cube3::Cube3},
    };
    use puzzle_geometry::ksolve::{KPUZZLE_3X3, KPUZZLE_4X4};

    #[test]
    fn test_identity_cycle_type() {
        let puzzle_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
            &puzzle_def,
            vec![vec![], vec![]],
            ZeroTable::generate(NullMeta),
        );
        let solutions = solver.solve::<[Cube3; 21]>();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].len(), 0);

        let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
            &puzzle_def,
            vec![vec![], vec![]],
            OrbitPruningTables::generate(
                OrbitPruningTablesGenerateMeta::new_with_table_types(
                    &puzzle_def,
                    0,
                    vec![
                        (OrbitPruningTableTy::Exact, StorageBackendTy::Zero),
                        (OrbitPruningTableTy::Exact, StorageBackendTy::Zero),
                    ],
                )
                .unwrap(),
            ),
        );
        let solutions = solver.solve::<[Cube3; 21]>();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].len(), 0);
    }

    #[test]
    fn test_single_quarter_turn() {
        let puzzle_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
            &puzzle_def,
            vec![
                vec![(4.try_into().unwrap(), false)],
                vec![(4.try_into().unwrap(), false)],
            ],
            ZeroTable::generate(NullMeta),
        );
        let solutions = solver.solve::<[Cube3; 21]>();
        assert_eq!(solutions.len(), 12);
        assert!(solutions.iter().all(|solution| solution.len() == 1));
    }

    #[test]
    fn test_single_half_turn() {
        let puzzle_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
            &puzzle_def,
            vec![
                vec![
                    (2.try_into().unwrap(), false),
                    (2.try_into().unwrap(), false),
                ],
                vec![
                    (2.try_into().unwrap(), false),
                    (2.try_into().unwrap(), false),
                ],
            ],
            ZeroTable::generate(NullMeta),
        );
        let solutions = solver.solve::<[Cube3; 21]>();
        assert_eq!(solutions.len(), 6);
        assert!(solutions.iter().all(|solution| solution.len() == 1));
    }

    #[test]
    fn test_optimal_subgroup_cycle() {
        let puzzle_def: PuzzleDef<Cube3> = (&KPUZZLE_3X3.clone().with_moves(&["F", "R", "U"]))
            .try_into()
            .unwrap();
        let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
            &puzzle_def,
            vec![
                vec![
                    (3.try_into().unwrap(), false),
                    (4.try_into().unwrap(), false),
                ],
                vec![(1.try_into().unwrap(), true), (8.try_into().unwrap(), true)],
            ],
            ZeroTable::generate(NullMeta),
        );
        let solutions = solver.solve::<[Cube3; 21]>();
        assert_eq!(solutions.len(), 22); // TODO: should be 24
        assert!(solutions.iter().all(|solution| solution.len() == 4));
    }

    #[test]
    fn test_control_optimal_cycle() {
        use std::time::Instant;

        let puzzle_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
            &puzzle_def,
            vec![
                vec![(1.try_into().unwrap(), true), (5.try_into().unwrap(), true)],
                vec![(1.try_into().unwrap(), true), (7.try_into().unwrap(), true)],
            ],
            ZeroTable::generate(NullMeta),
        );

        let start = Instant::now();
        let solutions = solver.solve::<[Cube3; 21]>();
        let duration = start.elapsed();
        eprintln!("Time to find optimal cycle: {:?}", duration);
        // for solution in solutions.iter() {
        //     for move_ in solution.iter() {
        //         print!("{} ", &move_.name);
        //     }
        //     println!();
        // }
        assert_eq!(solutions.len(), 260); // TODO: should be 480
        assert!(solutions.iter().all(|solution| solution.len() == 5));
    }

    #[allow(dead_code)]
    struct OptimalCycleTypeTest {
        moves_str: &'static str,
        expected_partial_count: usize,
        expected_count: usize,
    }

    #[test]
    fn test_many_optimal_cycles() {
        let cube3_def: PuzzleDef<HeapPuzzle> = (&*KPUZZLE_3X3).try_into().unwrap();

        let mut solver: CycleTypeSolver<HeapPuzzle, _> =
            CycleTypeSolver::new(&cube3_def, Vec::default(), ZeroTable::generate(NullMeta));

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
        let mut multi_bv = HeapPuzzle::new_multi_bv(&cube3_def.sorted_orbit_defs);

        for optimal_cycle_test in optimal_cycle_type_tests {
            let mut result_1 = solved.clone();
            let mut result_2 = solved.clone();
            let mut move_count = 0;
            for name in optimal_cycle_test.moves_str.split_whitespace() {
                let move_ = cube3_def.find_move(name).unwrap();
                result_2.replace_compose(
                    &result_1,
                    &move_.puzzle_state,
                    &cube3_def.sorted_orbit_defs,
                );
                std::mem::swap(&mut result_1, &mut result_2);
                move_count += 1;
            }

            let cycle_type =
                result_1.cycle_type(&cube3_def.sorted_orbit_defs, multi_bv.reusable_ref());
            solver.set_sorted_cycle_type(cycle_type);

            let solutions = solver.solve::<Vec<_>>();
            assert_eq!(solutions.len(), optimal_cycle_test.expected_partial_count);
            assert!(
                solutions
                    .iter()
                    .all(|solution| solution.len() == move_count)
            );
        }
    }

    #[test]
    fn test_big_cube_optimal_cycle() {
        let cube4_def: PuzzleDef<HeapPuzzle> = (&*KPUZZLE_4X4).try_into().unwrap();

        let mut solver: CycleTypeSolver<HeapPuzzle, _> =
            CycleTypeSolver::new(&cube4_def, Vec::default(), ZeroTable::generate(NullMeta));

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
        // only do 10 because this is slow
        let optimal_cycle_type_tests = &optimal_cycle_type_tests[0..5];

        let solved = cube4_def.new_solved_state();
        let mut multi_bv = HeapPuzzle::new_multi_bv(&cube4_def.sorted_orbit_defs);

        for optimal_cycle_test in optimal_cycle_type_tests {
            let mut result_1 = solved.clone();
            let mut result_2 = solved.clone();
            let mut move_count = 0;
            for name in optimal_cycle_test.moves_str.split_whitespace() {
                let move_ = cube4_def.find_move(name).unwrap();
                result_2.replace_compose(
                    &result_1,
                    &move_.puzzle_state,
                    &cube4_def.sorted_orbit_defs,
                );
                std::mem::swap(&mut result_1, &mut result_2);
                move_count += 1;
            }

            let cycle_type =
                result_1.cycle_type(&cube4_def.sorted_orbit_defs, multi_bv.reusable_ref());
            solver.set_sorted_cycle_type(cycle_type);

            let solutions = solver.solve::<Vec<_>>();
            assert_eq!(solutions.len(), optimal_cycle_test.expected_partial_count);
            assert!(
                solutions
                    .iter()
                    .all(|solution| solution.len() == move_count)
            );
        }
    }

    //     let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
    //         puzzle_def,
    //         canonical_fsm,
    //         vec![
    //             vec![(1.try_into().unwrap(), true), (5.try_into().unwrap(), true)],
    //             vec![(1.try_into().unwrap(), true), (1.try_into().unwrap(), true)],
    //         ],
    //         ZeroTable,
    //     );

    //     let start = Instant::now();
    //     let solutions = solver.solve::<[Cube3; 21]>();
    //     let duration = start.elapsed();
    //     eprintln!("Time to find 30-cycle: {:?}", duration);
    //     for solution in solutions.iter() {
    //         for move_ in solution.iter() {
    //             print!("{} ", &move_.name);
    //         }
    //         println!();
    //     }
    //     assert_eq!(solutions.len(), 0);
    // }
}
