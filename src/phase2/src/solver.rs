use super::{
    canonical_fsm::{CanonicalFSM, CanonicalFSMState, PuzzleCanonicalFSM},
    pruning::PruningTables,
    puzzle::{Move, PuzzleDef, PuzzleState},
    puzzle_state_history::{PuzzleStateHistory, PuzzleStateHistoryInterface},
};
use crate::{SliceViewMut, start, success, working};
use log::{Level, debug, info, log_enabled};
use std::{time::Instant, vec::IntoIter};

pub struct CycleTypeSolver<'id, P: PuzzleState<'id>, T: PruningTables<'id, P>> {
    puzzle_def: PuzzleDef<'id, P>,
    pruning_tables: T,
    canonical_fsm: PuzzleCanonicalFSM<'id, P>,
    search_strategy: SearchStrategy,
}

#[derive(PartialEq)]
pub enum SearchStrategy {
    FirstSolution,
    AllSolutions,
}

struct CycleTypeSolverMutable<'id, P: PuzzleState<'id>, H: PuzzleStateHistoryInterface<'id, P>> {
    puzzle_state_history: PuzzleStateHistory<'id, P, H>,
    aux_mem: P::AuxMem,
    solutions: Vec<Vec<usize>>,
    first_move_class_index: usize,
    nodes_visited: u64,
}

pub struct SolutionsIntoIter<'id, 'a, P: PuzzleState<'id>> {
    puzzle_def: &'a PuzzleDef<'id, P>,
    solutions: IntoIter<Vec<usize>>,
}

impl<'id, P: PuzzleState<'id>, T: PruningTables<'id, P>> CycleTypeSolver<'id, P, T> {
    pub fn new(
        puzzle_def: PuzzleDef<'id, P>,
        pruning_tables: T,
        search_strategy: SearchStrategy,
    ) -> Self {
        let canonical_fsm = (&puzzle_def).into();
        Self {
            puzzle_def,
            pruning_tables,
            canonical_fsm,
            search_strategy,
        }
    }

    pub fn into_puzzle_def_and_pruning_tables(self) -> (PuzzleDef<'id, P>, T) {
        (self.puzzle_def, self.pruning_tables)
    }

    fn search_for_solution<H: PuzzleStateHistoryInterface<'id, P>>(
        &self,
        mutable: &mut CycleTypeSolverMutable<'id, P, H>,
        current_fsm_state: CanonicalFSMState,
        entry_index: usize,
        root: bool,
        mut togo: u8,
    ) {
        if log_enabled!(Level::Debug) {
            mutable.nodes_visited += 1;
        }
        // SAFETY: This function calls `pop_stack` for every `push_stack` call.
        // Therefore, the `pop_stack` cannot be called more than `push_stack`.
        let last_puzzle_state = unsafe { mutable.puzzle_state_history.last_state_unchecked() };

        let est_remaining_cost = self.pruning_tables.permissible_heuristic(last_puzzle_state);
        if est_remaining_cost > togo {
            // TODO: what the heck does this do
            // https://github.com/cubing/twsearch/commit/a86177ac2bd462bb9d7d91af743e883449fbfb6b
            return;
        }

        let mut next_entry_index = entry_index + 1;
        // Tomas Rokicki's "sequence symmetry" optimization:
        // <https://github.com/cubing/twsearch/commit/7b1d62bd9d9d232fb4729c7227d5255deed9673c>
        //
        // SAFETY: `entry_index` starts at zero in the initial call, and
        // `B::initialize` guarantees that the first entry is bound. For every
        // recursive call, the puzzle history stack is pushed and `entry_index`
        // can only be incremented by 1. Therefore, `entry_index` is always
        // less than the number of entries in the puzzle state history and
        // always bound
        let move_index_prune_lt = unsafe {
            mutable
                .puzzle_state_history
                .move_index_unchecked(entry_index)
        };
        togo -= 1;
        for (move_index, move_) in self
            .puzzle_def
            .moves
            .iter()
            .enumerate()
            .skip(move_index_prune_lt)
        {
            // if self.search_strategy == SearchStrategy::FirstSolution && move_index == 2 && root {
            //     return false;
            // }
            let move_class_index = move_.move_class_index();
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
            }

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

            // We handle togo==0 inline to save the function call overhead
            if togo == 0 {
                // SAFETY: we just pushed something onto the stack
                let last_puzzle_state =
                    unsafe { mutable.puzzle_state_history.last_state_unchecked() };
                if last_puzzle_state.induces_sorted_cycle_type(
                    self.pruning_tables.sorted_cycle_type_slice_view(),
                    self.puzzle_def.sorted_orbit_defs_slice_view(),
                    mutable.aux_mem.slice_view_mut(),
                ) {
                    mutable
                        .solutions
                        .push(mutable.puzzle_state_history.create_move_history());
                }
            } else {
                // TODO: Actual IDA* takes the min of this bound and uses it; look into?
                self.search_for_solution(mutable, next_fsm_state, next_entry_index, false, togo);
            }

            if !mutable.solutions.is_empty()
                && self.search_strategy == SearchStrategy::FirstSolution
            {
                return;
            }

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

    pub fn solve<H: PuzzleStateHistoryInterface<'id, P>>(&self) -> SolutionsIntoIter<'id, '_, P> {
        info!(start!("Searching for phase2 solutions"));
        let start = Instant::now();

        let mut mutable: CycleTypeSolverMutable<P, H> = CycleTypeSolverMutable {
            puzzle_state_history: (&self.puzzle_def).into(),
            aux_mem: P::new_aux_mem(self.puzzle_def.sorted_orbit_defs_slice_view()),
            solutions: vec![],
            first_move_class_index: usize::default(),
            nodes_visited: 0,
        };
        // SAFETY: `H::initialize` when puzzle_state_history is created
        // guarantees that the first entry is bound
        let last_puzzle_state = unsafe { mutable.puzzle_state_history.last_state_unchecked() };
        let mut depth = self.pruning_tables.permissible_heuristic(last_puzzle_state);
        // Manually check depth 0 because the togo == 0 check was moved inside
        // of the main loop in `search_for_solution`.
        if depth == 0 {
            // The return values here don't matter since it's not used in the
            // below loop so we can get rid of `true` and `false`
            if last_puzzle_state.induces_sorted_cycle_type(
                self.pruning_tables.sorted_cycle_type_slice_view(),
                self.puzzle_def.sorted_orbit_defs_slice_view(),
                mutable.aux_mem.slice_view_mut(),
            ) {
                mutable
                    .solutions
                    .push(mutable.puzzle_state_history.create_move_history());
            }
            // The loop ends up incrementing `depth` so we do this manually
            depth = 1;
        }
        mutable
            .puzzle_state_history
            .resize_if_needed(depth as usize + 1);

        while mutable.solutions.is_empty() {
            debug!(working!("Searching depth {}..."), depth);
            let depth_start = Instant::now();
            self.search_for_solution(&mut mutable, CanonicalFSMState::default(), 0, true, depth);
            debug!(
                working!("Traversed {} nodes in {:.3}s"),
                mutable.nodes_visited,
                depth_start.elapsed().as_secs_f64()
            );
            mutable.nodes_visited = 0;
            mutable
                .puzzle_state_history
                .resize_if_needed(depth as usize + 1);
            depth += 1;
        }

        info!(
            success!("phase2 solutions found in {:.3}s"),
            start.elapsed().as_secs_f64()
        );
        debug!("");
        SolutionsIntoIter {
            puzzle_def: &self.puzzle_def,
            solutions: mutable.solutions.into_iter(),
        }
    }
}

impl<'id, 'a, P: PuzzleState<'id>> Iterator for SolutionsIntoIter<'id, 'a, P> {
    type Item = Vec<&'a Move<'id, P>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.solutions.next().map(|solution| {
            solution
                .into_iter()
                .map(|move_index| &self.puzzle_def.moves[move_index])
                .collect()
        })
    }
}
