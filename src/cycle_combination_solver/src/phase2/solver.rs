use super::{
    canonical_fsm::{self, CanonicalFSM, CanonicalFSMState},
    pruning::PruningTable,
    puzzle::{Move, MultiBvInterface, OrientedPartition, PuzzleDef, PuzzleState},
    puzzle_state_history::{PuzzleStateHistory, PuzzleStateHistoryInterface},
};

pub struct CycleTypeSolver<P: PuzzleState, T: PruningTable<P>> {
    puzzle_def: PuzzleDef<P>,
    canonical_fsm: CanonicalFSM<P>,
    sorted_cycle_type: Vec<OrientedPartition>,
    pruning_table: T,
}

struct CycleTypeSolverMutable<P: PuzzleState, H: PuzzleStateHistoryInterface<P>> {
    puzzle_state_history: PuzzleStateHistory<P, H>,
    multi_bv: P::MultiBv,
    solutions: Vec<Box<[Move<P>]>>,
}

impl<P: PuzzleState, T: PruningTable<P>> CycleTypeSolver<P, T> {
    pub fn new(
        puzzle_def: PuzzleDef<P>,
        canonical_fsm: CanonicalFSM<P>,
        sorted_cycle_type: Vec<OrientedPartition>,
        pruning_table: T,
    ) -> Self {
        Self {
            puzzle_def,
            canonical_fsm,
            sorted_cycle_type,
            pruning_table,
        }
    }

    fn search_for_solution<H: PuzzleStateHistoryInterface<P>>(
        &self,
        mutable: &mut CycleTypeSolverMutable<P, H>,
        current_fsm_state: CanonicalFSMState,
        entry_index: usize,
        sofar_cost: u8,
        // TODO: make this `togo` and descending (see sc) and vcube does IDA*
        // fundamentally differently, ie it returns est_remaining_cost instead
        // of est_goal_cost.
        cost_bound: u8,
    ) -> u8 {
        // SAFETY: This function calls `pop_stack` for every `push_stack` call.
        // Therefore, the `pop_stack` cannot be called more than `push_stack`.
        let last_puzzle_state = unsafe { mutable.puzzle_state_history.last_state_unchecked() };
        let est_remaining_cost = self.pruning_table.permissible_heuristic(last_puzzle_state);
        let est_goal_cost = sofar_cost + est_remaining_cost;

        if est_goal_cost > cost_bound {
            return est_goal_cost;
        }
        if last_puzzle_state.induces_sorted_cycle_type(
            &self.sorted_cycle_type,
            mutable.multi_bv.reusable_ref(),
            &self.puzzle_def.sorted_orbit_defs,
        ) {
            mutable.solutions.push(
                mutable
                    .puzzle_state_history
                    .create_move_history(&self.puzzle_def),
            );
        }

        let mut min_next_est_goal_cost = u8::MAX;
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
        // TODO: make start nonzero
        for move_index in start..self.puzzle_def.moves.len() {
            let move_class_index = self.puzzle_def.moves[move_index].move_class_index;
            let Some(next_fsm_state) = self
                .canonical_fsm
                .next_state(current_fsm_state, move_class_index)
            else {
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
                    .push_stack_unchecked(move_index, &self.puzzle_def);
            }
            let next_est_goal_cost = self.search_for_solution(
                mutable,
                next_fsm_state,
                next_entry_index,
                sofar_cost + 1,
                cost_bound,
            );
            mutable.puzzle_state_history.pop_stack();
            next_entry_index = 0;

            // TODO: BPMX optimization
            // if next_est_goal_cost.saturating_sub(1) > est_goal_cost {
            //     est_goal_cost = next_est_goal_cost.saturating_sub(1);
            //     if est_goal_cost > cost_bound {
            //         return est_goal_cost;
            //     }
            // }

            min_next_est_goal_cost = min_next_est_goal_cost.min(next_est_goal_cost);
        }
        min_next_est_goal_cost
    }

    pub fn solve<H: PuzzleStateHistoryInterface<P>>(&self) -> Vec<Box<[Move<P>]>> {
        let mut mutable: CycleTypeSolverMutable<P, H> = CycleTypeSolverMutable {
            puzzle_state_history: (&self.puzzle_def).into(),
            multi_bv: P::new_multi_bv(&self.puzzle_def.sorted_orbit_defs),
            solutions: vec![],
        };
        let mut cost_bound = self.pruning_table.permissible_heuristic(
            // SAFETY: `B::initialize` guarantees that the first entry is bound
            unsafe { mutable.puzzle_state_history.last_state_unchecked() },
        );
        mutable
            .puzzle_state_history
            .resize_if_needed(cost_bound as usize + 1);
        while mutable.solutions.is_empty() {
            println!("Searching depth {}...", cost_bound);
            cost_bound = self.search_for_solution(
                &mut mutable,
                CanonicalFSMState::default(),
                0,
                0,
                cost_bound,
            );
            mutable
                .puzzle_state_history
                .resize_if_needed(cost_bound as usize + 1);
        }
        mutable.solutions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::{pruning::ZeroTable, puzzle::cube3::Cube3};
    use puzzle_geometry::ksolve::KPUZZLE_3X3;

    #[test]
    fn test_identity_cycle_type() {
        let puzzle_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let canonical_fsm = (&puzzle_def).try_into().unwrap();
        let solver: CycleTypeSolver<Cube3, _> =
            CycleTypeSolver::new(puzzle_def, canonical_fsm, vec![vec![], vec![]], ZeroTable);
        let solutions = solver.solve::<[Cube3; 21]>();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].len(), 0);
    }

    #[test]
    fn test_single_quarter_turn() {
        let puzzle_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let canonical_fsm = (&puzzle_def).try_into().unwrap();
        let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
            puzzle_def,
            canonical_fsm,
            vec![
                vec![(4.try_into().unwrap(), false)],
                vec![(4.try_into().unwrap(), false)],
            ],
            ZeroTable,
        );
        let solutions = solver.solve::<[Cube3; 21]>();
        assert_eq!(solutions.len(), 12);
        assert!(solutions.iter().all(|solution| solution.len() == 1));
    }

    #[test]
    fn test_single_half_turn() {
        let puzzle_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let canonical_fsm = (&puzzle_def).try_into().unwrap();
        let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
            puzzle_def,
            canonical_fsm,
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
            ZeroTable,
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
        let canonical_fsm = (&puzzle_def).try_into().unwrap();
        let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
            puzzle_def,
            canonical_fsm,
            vec![
                vec![
                    (3.try_into().unwrap(), false),
                    (4.try_into().unwrap(), false),
                ],
                vec![(1.try_into().unwrap(), true), (8.try_into().unwrap(), true)],
            ],
            ZeroTable,
        );
        let solutions = solver.solve::<[Cube3; 21]>();
        assert_eq!(solutions.len(), 22); // TODO: should be 24
        assert!(solutions.iter().all(|solution| solution.len() == 4));
    }

    #[test]
    fn test_optimal_cycle() {
        use std::time::Instant;

        let puzzle_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let canonical_fsm = (&puzzle_def).try_into().unwrap();
        let solver: CycleTypeSolver<Cube3, _> = CycleTypeSolver::new(
            puzzle_def,
            canonical_fsm,
            vec![
                vec![(1.try_into().unwrap(), true), (5.try_into().unwrap(), true)],
                vec![(1.try_into().unwrap(), true), (7.try_into().unwrap(), true)],
            ],
            ZeroTable,
        );

        let start = Instant::now();
        let solutions = solver.solve::<[Cube3; 21]>();
        let duration = start.elapsed();
        println!("Time to find optimal cycle: {:?}", duration);
        // for solution in solutions.iter() {
        //     for move_ in solution.iter() {
        //         print!("{} ", &move_.name);
        //     }
        //     println!();
        // }
        assert_eq!(solutions.len(), 260); // TODO: should be 480
        assert!(solutions.iter().all(|solution| solution.len() == 5));
    }
}
