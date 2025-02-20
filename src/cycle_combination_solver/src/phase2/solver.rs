use super::{
    pruning::PruningTable,
    puzzle::{KSolveConversionError, Move, OrientedPartition, PuzzleDef, PuzzleState},
};

pub struct CycleTypeSolver<T: PruningTable, P: PuzzleState> {
    puzzle_def: PuzzleDef<P>,
    cycle_type: Vec<OrientedPartition>,
    pruning_table: T,
}

enum SearchResult {
    Found,
    NewBound(u8),
}

impl<T: PruningTable, P: PuzzleState> CycleTypeSolver<T, P> {
    pub fn new(
        puzzle_def: PuzzleDef<P>,
        cycle_type: Vec<OrientedPartition>,
        pruning_table: T,
    ) -> Self {
        Self {
            puzzle_def,
            cycle_type,
            pruning_table,
        }
    }

    // fn search_for_solution(
    //     &self,
    //     curr_path: &mut MoveSequence,
    //     last_state: &CubeState,
    //     g: u8,
    //     bound: u8,
    // ) -> SearchResult {
    //     let last_h = self.pruning_tables.compute_h_value(last_state);
    //     let f = g + last_h;
    //     if f > bound {
    //         SearchResult::NewBound(f)
    //     } else if last_state.induces_cycle_type(&self.target_cycle_type, self.multi_bv.as_mut()) {
    //         // yay it's solved!
    //         SearchResult::Found
    //     } else {
    //         let mut min = u8::MAX;
    //         let allowed_moves = curr_path.allowed_moves_after_seq();
    //         for m in cube::ALL_MOVES
    //             .iter()
    //             .filter(|mo| ((1 << cube::get_basemove_pos(mo.basemove)) & allowed_moves) == 0)
    //         {
    //             if !curr_path.is_empty() {
    //                 let last_move = curr_path[curr_path.len() - 1];
    //                 if last_move.basemove == m.basemove {
    //                     continue;
    //                 }
    //             }
    //             curr_path.push(*m);
    //             let next_state = last_state.apply_move_instance(m);
    //             let t = self.search_for_solution(curr_path, &next_state, g + 1, bound);
    //             match t {
    //                 SearchResult::Found => return SearchResult::Found,
    //                 SearchResult::NewBound(b) => {
    //                     min = std::cmp::min(b, min);
    //                 }
    //             };
    //             curr_path.pop();
    //         }
    //         SearchResult::NewBound(min)
    //     }
    // }

    // // TODO: all solutions
    // pub fn solve(&self) -> Result<Vec<Move<P>>, KSolveConversionError> {
    //     let start_state = self.puzzle_def.solved_state()?;

    //     // initial lower bound on number of moves needed to solve start state
    //     let mut bound = self.pruning_tables.compute_h_value(&start_state);
    //     let mut path: MoveSequence = MoveSequence::default();
    //     loop {
    //         println!("Searching depth {}...", bound);
    //         match self.search_for_solution(&mut path, &start_state, 0, bound) {
    //             SearchResult::Found => {
    //                 break;
    //             }
    //             SearchResult::NewBound(t) => {
    //                 bound = t;
    //             }
    //         }
    //     }
    //     path
    // }
}
