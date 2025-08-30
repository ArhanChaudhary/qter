use super::{
    canonical_fsm::{CanonicalFSM, CanonicalFSMState, PuzzleCanonicalFSM},
    pruning::PruningTables,
    puzzle::{Move, PuzzleDef, PuzzleState},
    puzzle_state_history::{PuzzleStateHistory, StackedPuzzleStateHistory},
};
use crate::{SliceViewMut, puzzle::AuxMem, start, success, working};
use log::{Level, debug, info, log_enabled};
use std::{time::Instant, vec::IntoIter};
use thiserror::Error;

pub struct CycleTypeSolver<'id, P: PuzzleState<'id>, T: PruningTables<'id, P>> {
    puzzle_def: PuzzleDef<'id, P>,
    pruning_tables: T,
    canonical_fsm: PuzzleCanonicalFSM<'id, P>,
    max_solution_length: Option<usize>,
    search_strategy: SearchStrategy,
}

struct CycleTypeSolverMutable<'id, P: PuzzleState<'id>, H: PuzzleStateHistory<'id, P>> {
    puzzle_state_history: StackedPuzzleStateHistory<'id, P, H>,
    aux_mem: AuxMem<'id>,
    solutions: Vec<Vec<usize>>,
    first_move_class_index: usize,
    nodes_visited: u64,
}

#[derive(Error, Debug)]
pub enum CycleTypeSolverError {
    #[error("A deep search still did not find a solution. It is unlikely that one exists")]
    SolutionDoesNotExist,
    #[error("Max solution length exceeded")]
    MaxSolutionLengthExceeded,
    #[error("Time limit exceeded")]
    TimeLimitExceeded,
}

/// The return type of the IDA* recursion function. It maintains the
/// soft-invariant that zero means a solution has been found, hence
/// `AdmissibleGoalHeuristic::SOLVED`.
#[derive(PartialEq, Copy, Clone)]
struct AdmissibleGoalHeuristic(u8);

impl AdmissibleGoalHeuristic {
    const SOLVED: Self = Self(0);
}

// TODO: is this worth making a const generic?
#[derive(PartialEq)]
pub enum SearchStrategy {
    FirstSolution,
    AllSolutions,
}

impl<'id, P: PuzzleState<'id>, H: PuzzleStateHistory<'id, P>> CycleTypeSolverMutable<'id, P, H> {
    fn found_solution(&self) -> bool {
        !self.solutions.is_empty()
    }
}

#[derive(Debug)]
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
            max_solution_length: None,
            search_strategy,
        }
    }

    #[must_use]
    pub fn with_max_solution_length(mut self, max_solution_length: usize) -> Self {
        self.max_solution_length = Some(max_solution_length);
        self
    }

    pub fn into_puzzle_def_and_pruning_tables(self) -> (PuzzleDef<'id, P>, T) {
        (self.puzzle_def, self.pruning_tables)
    }

    /// A highly optimized [iterative deepening A*][IDA] search algorithm. We
    /// employ a number of techniques, some specific to a cycle type solver
    /// only:
    ///
    /// - We reduce the branching factor by using a finite state machine of
    ///   non-commutative moves.
    /// - We embed a "sequence symmetry" optimization into search, which takes
    ///   advantage of the properties of conjugacy classes.
    /// - We disallow the same move class at the beginning and end to optimize
    ///   the last depth in the search.
    /// - We promote the heuristic to one if the pruning value is zero.
    /// - We use pathmax to prune nodes with large child pruning values.
    ///
    /// The return value is an admissible goal heuristic. That is, it is a
    /// lower bound on the number of moves required to find the solution state
    /// at the exact node. When this lower bound is equal to zero, that means
    /// the node is a solution.
    ///
    /// [IDA]: https://en.wikipedia.org/wiki/Iterative_deepening_A*
    fn search_for_solution<H: PuzzleStateHistory<'id, P>>(
        &self,
        mutable: &mut CycleTypeSolverMutable<'id, P, H>,
        current_fsm_state: CanonicalFSMState,
        entry_index: usize,
        mut permitted_cost: u8,
    ) -> AdmissibleGoalHeuristic {
        if log_enabled!(Level::Debug) {
            mutable.nodes_visited += 1;
        }
        // SAFETY: This function calls `pop_stack` for every `push_stack` call.
        // Therefore, the `pop_stack` cannot be called more than `push_stack`.
        let last_puzzle_state = unsafe { mutable.puzzle_state_history.last_state_unchecked() };

        let admissible_prune_cost = self.pruning_tables.admissible_heuristic(last_puzzle_state);
        if admissible_prune_cost > permitted_cost {
            // Note that `admissible_prune_heuristic` is impossible to be zero
            // here, so the enum instantiation is valid
            return AdmissibleGoalHeuristic(admissible_prune_cost);
        }

        // Sequence symmetry optimization, first observed by [Tomas Rokicki][ss],
        // and slightly improved by this implementation. Some solution to Phase2
        // A B C D conjugated by A^-1 yields A^-1 (A B C D) A = B C D A, which
        // we observe to be a rotation of the original sequence. One of the
        // properties of conjugate elements is that they cannot be distinguished
        // by using only the group structure, and this includes the cycle
        // structure we are solving for. Recursively applying this conjugation
        // forms an equivalence class based on the rotations of sequences, so we
        // search only a single representative sequence to avoid duplicate work.
        // We choose the representative as the lexicographically minimal
        // sequence because this restriction is easy to embed in IDA* search.
        //
        // [ss]: https://github.com/cubing/twsearch/commit/7b1d62bd9d9d232fb4729c7227d5255deed9673c
        //
        // Let us define:
        //
        // - `X` as the node about to be explored in the recursive step of IDA*
        // - The inequality symbols lexicographically
        // - `HISTORY(i)` as a function that returns the one-indexed move in the
        //   current move history.
        // - The integer `i`, initialized to zero
        //
        // The sequence symmetry optimization recursively goes through the
        // following decision tree to ensure it only ever searches the
        // lexicographically minimal sequence by their rotation:
        //
        // - If i == 0 or X > HISTORY(i), then append X to the move history and
        //   set `i` to one in the next recursion.
        // - If X == HISTORY(i), then append X to the move history and increment
        //   `i` in the next recursion.
        // - If X < HISTORY(i), then prune X. X can be rotated to the front of
        //   the sequence to produce something lexicographically lesser.
        //
        // SAFETY: `entry_index` starts at zero in the initial call, and
        // `B::initialize` guarantees that the first entry is bound. For every
        // recursive call, the puzzle history stack is pushed and `entry_index`
        // can only be incremented by 1 at most. Therefore, `entry_index` is
        // always less than the number of entries in the puzzle state history
        // and always bound
        let move_index_prune_lt = unsafe {
            mutable
                .puzzle_state_history
                .move_index_unchecked(entry_index)
        };
        permitted_cost -= 1;
        for (move_index, move_) in self
            .puzzle_def
            .moves
            .iter()
            .enumerate()
            // We are at the "X < HISTORY(i)" case described earlier. Pruning
            // all of the child nodes is as simple as skipping a "HISTORY(i)"
            // number of children from the beginning.
            .skip(move_index_prune_lt)
        {
            // if self.search_strategy == SearchStrategy::FirstSolution && move_index == 2 && is_root {
            //     return AdmissibleGoalHeuristic::SOLVED;
            // }

            let move_class_index = move_.move_class_index();
            // This is only ever true at the root node
            let is_root = entry_index == 0;
            // This branch should have high predictability
            if is_root {
                mutable.first_move_class_index = move_class_index;
            // We take advantage of the fact that the shortest sequence can
            // never start and end with the moves in the same move class.
            // Otherwise the end could be rotated to the start and combined
            // together, thus contradicting that assumption
            //
            // TODO: investigate optimizing this once phase2 becomes more mature
            // see: https://discord.com/channels/772576325897945119/1326029986578038784/1411433393387999345
            } else if permitted_cost == 0 && move_class_index == mutable.first_move_class_index {
                continue;
            }

            // We use a canonical FSM to enforce a total ordering of commutating
            // moves. For example, U D and D U produce equivalent states, so
            // there is no point in searching both
            let next_fsm_state = self
                .canonical_fsm
                .next_state(current_fsm_state, move_class_index);
            if next_fsm_state.is_none() {
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

            // We handle when `permitted_cost == 0` (leaf node) inline to save
            // the recursive function call overhead otherwise incurred
            let child_admissible_goal_heuristic = if permitted_cost == 0 {
                // [Chen Shuang][cs] doesn't count the maxdepth nodes as
                // visited, so we won't either
                //
                // [cs]: https://github.com/cs0x7f/cube_solver_test/blob/bd78072c49f8a01aee16d592400cbd6e2bacca93/vcube.patch#L22

                // SAFETY: we just pushed something onto the stack
                let last_puzzle_state =
                    unsafe { mutable.puzzle_state_history.last_state_unchecked() };
                if last_puzzle_state.induces_sorted_cycle_type(
                    self.pruning_tables.sorted_cycle_type_ref(),
                    self.puzzle_def.sorted_orbit_defs_ref(),
                    mutable.aux_mem.slice_view_mut(),
                ) {
                    mutable
                        .solutions
                        .push(mutable.puzzle_state_history.create_move_history());
                    AdmissibleGoalHeuristic::SOLVED
                } else {
                    // If this node resulted in no solution, then we are at
                    // least one step away from a solution
                    AdmissibleGoalHeuristic(1)
                }
            } else {
                // This optimizes to branchless code
                let next_entry_index = if move_index == move_index_prune_lt {
                    // We are at the "X == HISTORY(i)" case described earlier.
                    // We increment the move history index in the next
                    // recursion.
                    entry_index + 1
                } else {
                    // We are at the "X > HISTORY(i)" case described earlier.
                    // We set the move history index to one in the next
                    // recursion.
                    1
                };
                self.search_for_solution(mutable, next_fsm_state, next_entry_index, permitted_cost)
            };

            // If we've found a solution, and our search strategy is to
            // find the first solution, we instantly terminate. No more
            // processing will occur once this returns
            if self.search_strategy == SearchStrategy::FirstSolution
                // We cannot use `child_admissible_goal_heuristic == 0` because
                // the following assert fails when placed:
                // TODO: formalize why
                // assert_eq!(mutable.found_solution(), child_admissible_goal_heuristic == 0);
                && mutable.found_solution()
            {
                // We don't care about preserving `mutable.puzzle_state_history`
                // anymore because there is no further processing
                return AdmissibleGoalHeuristic::SOLVED;
            }

            mutable.puzzle_state_history.pop_stack();

            // Pathmax optimization. If the child node has a large pruning
            // value, then we can set the current node cost to that value minus
            // one and re-prune. This larger value is still admissible because
            // it is one less then a known lower bound.
            //
            // Note that this is only effective when the heuristics are
            // **inconsistent**, or when the pruning table entry is the minimum
            // of two or more other values. With exact tables, this if statement
            // will never run, which should be good for the branch predictor.
            if
            // Assume the current node to be the child node heuristic minus one,
            // and the permitted cost plus one because we subtracted one from it
            // before entering this loop.
            //
            // IMPORTANT: We carry over the minus one to the right side to
            // create plus two. It must be written with a plus two to prevent
            // overflow.
            child_admissible_goal_heuristic.0 > permitted_cost + 2 {
                // The child node heuristic minus one cannot be zero and break
                // the zero invariant because 1 > X + 2 cannot be true for any
                // u8 (we don't care about 254). It cannot be zero either and
                // overflow for the same reason.
                return AdmissibleGoalHeuristic(child_admissible_goal_heuristic.0 - 1);
            }

            // TODO: look into taking the min of all the child nodes
        }
        AdmissibleGoalHeuristic(
            // This optimizes to branchless code
            if admissible_prune_cost == 0 {
                // If this node resulted in no solution, then we are at least
                // one step away from a solution.
                1
            } else {
                admissible_prune_cost
            },
        )
    }

    /// Run qter's cycle combination solver.
    ///
    /// # Errors
    ///
    /// The solver will fail if it cannot find a solution. See
    /// `CycleTypeSolverError`.
    pub fn solve<H: PuzzleStateHistory<'id, P>>(
        &self,
    ) -> Result<SolutionsIntoIter<'id, '_, P>, CycleTypeSolverError> {
        info!(start!("Searching for phase2 solutions"));
        let start = Instant::now();

        let mut mutable: CycleTypeSolverMutable<P, H> = CycleTypeSolverMutable {
            puzzle_state_history: (&self.puzzle_def).into(),
            aux_mem: P::new_aux_mem(self.puzzle_def.sorted_orbit_defs_ref()),
            solutions: vec![],
            first_move_class_index: 0,
            nodes_visited: 0,
        };
        // SAFETY: `H::initialize` when puzzle_state_history is created
        // guarantees that the first entry is bound
        let last_puzzle_state = unsafe { mutable.puzzle_state_history.last_state_unchecked() };
        let mut depth = self.pruning_tables.admissible_heuristic(last_puzzle_state);
        // Manually check depth 0 because the `permitted_cost == 0` check was
        // moved inside of the main loop in `search_for_solution`.
        if depth == 0 {
            debug!(working!("Searching depth {}..."), depth);
            let depth_start = Instant::now();
            // The return values here don't matter since it's not used in the
            // below loop so we can get rid of `true` and `false`
            if last_puzzle_state.induces_sorted_cycle_type(
                self.pruning_tables.sorted_cycle_type_ref(),
                self.puzzle_def.sorted_orbit_defs_ref(),
                mutable.aux_mem.slice_view_mut(),
            ) {
                mutable
                    .solutions
                    .push(mutable.puzzle_state_history.create_move_history());
            }
            debug!(
                working!("Traversed {} nodes in {:.3}s"),
                mutable.nodes_visited,
                depth_start.elapsed().as_secs_f64()
            );
            // The loop increments `depth` so we do this manually
            depth = 1;
        }

        if !mutable.found_solution() {
            if depth == u8::MAX {
                return Err(CycleTypeSolverError::SolutionDoesNotExist);
            }
            if let Some(max_solution_length) = self.max_solution_length
                && depth as usize > max_solution_length
            {
                return Err(CycleTypeSolverError::MaxSolutionLengthExceeded);
            }
            mutable.nodes_visited = 0;
            mutable
                .puzzle_state_history
                .resize_if_needed(depth as usize);
            loop {
                debug!(working!("Searching depth {}..."), depth);
                let depth_start = Instant::now();
                // `entry_index` must be zero here so the root level so sequence
                // symmetry doesn't access OOB move history entries.
                self.search_for_solution(
                    &mut mutable,
                    CanonicalFSMState::default(),
                    // Remember that `i` must be initialized to zero for the
                    // sequence symmetry optimization to work.
                    0,
                    depth,
                );
                debug!(
                    working!("Traversed {} nodes in {:.3}s"),
                    mutable.nodes_visited,
                    depth_start.elapsed().as_secs_f64()
                );
                if mutable.found_solution() {
                    break;
                }
                depth += 1;
                // During pathmax we increment the depth by one, so we ensure it
                // cannot overflow
                if depth == u8::MAX {
                    return Err(CycleTypeSolverError::SolutionDoesNotExist);
                }
                if let Some(max_solution_length) = self.max_solution_length
                    && depth as usize > max_solution_length
                {
                    return Err(CycleTypeSolverError::MaxSolutionLengthExceeded);
                }
                mutable.nodes_visited = 0;
                mutable
                    .puzzle_state_history
                    .resize_if_needed(depth as usize);
            }
        }

        info!(
            success!("phase2 solutions found in {:.3}s at depth {}"),
            start.elapsed().as_secs_f64(),
            depth
        );
        debug!("");
        Ok(SolutionsIntoIter {
            puzzle_def: &self.puzzle_def,
            solutions: mutable.solutions.into_iter(),
        })
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
