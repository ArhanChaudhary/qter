use super::{
    pruning::PruningTable,
    puzzle::{Move, MultiBvInterface, OrientedPartition, PuzzleDef, PuzzleState},
    puzzle_state_history::{PuzzleStateHistory, PuzzleStateHistoryInterface},
};

pub struct CycleTypeSolver<P: PuzzleState, T: PruningTable<P>, B: PuzzleStateHistoryInterface<P>> {
    puzzle_def: PuzzleDef<P>,
    sorted_cycle_type: Vec<OrientedPartition>,
    pruning_table: T,
    _marker: std::marker::PhantomData<B>,
}

struct CycleTypeSolverMutable<P: PuzzleState, B: PuzzleStateHistoryInterface<P>> {
    puzzle_state_history: PuzzleStateHistory<P, B>,
    multi_bv: <P as PuzzleState>::MultiBv,
    solutions: Vec<Box<[Move<P>]>>,
}

impl<P: PuzzleState, T: PruningTable<P>, B: PuzzleStateHistoryInterface<P>>
    CycleTypeSolver<P, T, B>
{
    pub fn new(
        puzzle_def: PuzzleDef<P>,
        sorted_cycle_type: Vec<OrientedPartition>,
        pruning_table: T,
    ) -> Self {
        Self {
            puzzle_def,
            sorted_cycle_type,
            pruning_table,
            _marker: std::marker::PhantomData,
        }
    }

    fn search_for_solution(
        &self,
        mutable: &mut CycleTypeSolverMutable<P, B>,
        move_index: usize,
        sofar_cost: u8,
        cost_bound: u8,
    ) -> u8 {
        let last_puzzle_state = mutable.puzzle_state_history.last_state();
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
        // FIXME: this doesn't cover every symmetric sequence
        let mut next_move_index = move_index + 1;
        let start = mutable.puzzle_state_history.get_move(move_index);
        for move_index in start..self.puzzle_def.moves.len() {
            // if not a canonical sequence continue and set next_move_index to 0
            mutable
                .puzzle_state_history
                .push_stack(move_index, &self.puzzle_def);
            let next_est_goal_cost =
                self.search_for_solution(mutable, next_move_index, sofar_cost + 1, cost_bound);
            mutable.puzzle_state_history.pop_stack();
            next_move_index = 0;

            // BPMX optimization
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

    pub fn solve(&self) -> Vec<Box<[Move<P>]>> {
        let mut mutable = CycleTypeSolverMutable {
            puzzle_state_history: (&self.puzzle_def).into(),
            multi_bv: P::new_multi_bv(&self.puzzle_def.sorted_orbit_defs),
            solutions: vec![],
        };
        let mut cost_bound = self
            .pruning_table
            .permissible_heuristic(mutable.puzzle_state_history.last_state());
        while mutable.solutions.is_empty() {
            println!("Searching depth {}...", cost_bound);
            cost_bound = self.search_for_solution(&mut mutable, 0, 0, cost_bound);
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
        let solver: CycleTypeSolver<Cube3, _, [Cube3; 21]> = CycleTypeSolver::new(
            (&*KPUZZLE_3X3).try_into().unwrap(),
            vec![vec![], vec![]],
            ZeroTable,
        );
        let solutions = solver.solve();
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].len(), 0);
    }

    #[test]
    fn test_single_quarter_turn() {
        let solver: CycleTypeSolver<Cube3, _, [Cube3; 21]> = CycleTypeSolver::new(
            (&*KPUZZLE_3X3).try_into().unwrap(),
            vec![
                vec![(4.try_into().unwrap(), false)],
                vec![(4.try_into().unwrap(), false)],
            ],
            ZeroTable,
        );
        let solutions = solver.solve();
        assert_eq!(solutions.len(), 12);
        assert!(solutions.iter().all(|solution| solution.len() == 1));
    }

    #[test]
    fn test_single_half_turn() {
        let solver: CycleTypeSolver<Cube3, _, [Cube3; 21]> = CycleTypeSolver::new(
            (&*KPUZZLE_3X3).try_into().unwrap(),
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
        let solutions = solver.solve();
        assert_eq!(solutions.len(), 6);
        assert!(solutions.iter().all(|solution| solution.len() == 1));
    }

    #[test]
    fn test_210_order() {
        let solver: CycleTypeSolver<Cube3, _, [Cube3; 21]> = CycleTypeSolver::new(
            (&*KPUZZLE_3X3).try_into().unwrap(),
            vec![
                vec![(1.try_into().unwrap(), true), (5.try_into().unwrap(), true)],
                vec![(1.try_into().unwrap(), true), (7.try_into().unwrap(), true)],
            ],
            ZeroTable,
        );
        let solutions = solver.solve();
        // for solution in solutions.iter() {
        //     for move_ in solution.iter() {
        //         print!("{} ", &move_.name);
        //     }
        //     println!();
        // }
        assert_eq!(solutions.len(), 440);
        assert!(solutions.iter().all(|solution| solution.len() == 5));
    }
}
