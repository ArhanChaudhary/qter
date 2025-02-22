use super::{
    pruning::PruningTable,
    puzzle::{Move, MultiBvInterface, OrientedPartition, PuzzleDef, PuzzleState},
    puzzle_state_history::{PuzzleStateHistory, PuzzleStateHistoryBuf},
};

pub struct CycleTypeSolver<P: PuzzleState, T: PruningTable<P>, B: PuzzleStateHistoryBuf<P>> {
    puzzle_def: PuzzleDef<P>,
    sorted_cycle_type: Vec<OrientedPartition>,
    pruning_table: T,
    _marker: std::marker::PhantomData<B>,
}

struct CycleTypeSolverMutable<P: PuzzleState, B: PuzzleStateHistoryBuf<P>> {
    puzzle_state_history: PuzzleStateHistory<P, B>,
    multi_bv: <P as PuzzleState>::MultiBv,
    solutions: Vec<Box<[Move<P>]>>,
}

impl<P: PuzzleState, T: PruningTable<P>, B: PuzzleStateHistoryBuf<P>> CycleTypeSolver<P, T, B> {
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
        sofar_cost: u8,
        cost_bound: u8,
    ) -> u8 {
        let last_puzzle_state = mutable.puzzle_state_history.last_state();
        let remaining_cost = self.pruning_table.permissible_heuristic(last_puzzle_state);
        let goal_cost = sofar_cost + remaining_cost;
        if goal_cost > cost_bound {
            return goal_cost;
        }
        if last_puzzle_state.induces_sorted_cycle_type(
            &self.sorted_cycle_type,
            mutable.multi_bv.reusable_ref(),
            &self.puzzle_def.sorted_orbit_defs,
        ) {
            mutable
                .solutions
                .push(mutable.puzzle_state_history.move_sequence(&self.puzzle_def));
        }
        let mut next_cost_bound = u8::MAX;
        // let allowed_moves = self.puzzle_state_history.allowed_moves_after_seq();
        // for m in cube::ALL_MOVES
        //     .iter()
        //     .filter(|mo| ((1 << cube::get_basemove_pos(mo.basemove)) & allowed_moves) == 0)
        // TODO: fetch cost_bound from recursive step and return early
        for move_ in self.puzzle_def.moves.iter() {
            mutable
                .puzzle_state_history
                .push_stack(&move_.puzzle_state, &self.puzzle_def);
            next_cost_bound = self
                .search_for_solution(mutable, sofar_cost + 1, cost_bound)
                .min(next_cost_bound);
            mutable.puzzle_state_history.pop_stack();
        }
        next_cost_bound
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
            cost_bound = self.search_for_solution(&mut mutable, 0, cost_bound);
        }
        mutable.solutions
    }
}
