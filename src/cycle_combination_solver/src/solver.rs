use super::{
    canonical_fsm::{CanonicalFSM, CanonicalFSMState, PuzzleCanonicalFSM},
    pruning::PruningTables,
    puzzle::{Move, PuzzleDef, PuzzleState},
    puzzle_state_history::{PuzzleStateHistory, StackedPuzzleStateHistory},
};
use crate::{puzzle::AuxMem, start, success, working};
use itertools::Itertools;
use log::{Level, debug, info, log_enabled};
use std::{borrow::Cow, time::Instant, vec::IntoIter};
use thiserror::Error;

pub struct CycleStructureSolver<'id, P: PuzzleState<'id>, T: PruningTables<'id, P>> {
    puzzle_def: PuzzleDef<'id, P>,
    pruning_tables: T,
    canonical_fsm: PuzzleCanonicalFSM<'id, P>,
    max_solution_length: Option<usize>,
    search_strategy: SearchStrategy,
}

struct CycleStructureSolverMutable<'id, P: PuzzleState<'id>, H: PuzzleStateHistory<'id, P>> {
    puzzle_state_history: StackedPuzzleStateHistory<'id, P, H>,
    aux_mem: AuxMem<'id>,
    solutions: Vec<Vec<usize>>,
    first_move_class_index: usize,
    nodes_visited: u64,
}

#[derive(Error, Debug)]
pub enum CycleStructureSolverError {
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

impl<'id, P: PuzzleState<'id>, H: PuzzleStateHistory<'id, P>>
    CycleStructureSolverMutable<'id, P, H>
{
    fn found_solution(&self) -> bool {
        !self.solutions.is_empty()
    }
}

#[derive(Debug)]
pub struct SolutionsIntoIter<'id, 'a, P: PuzzleState<'id>> {
    puzzle_def: &'a PuzzleDef<'id, P>,
    result_1: P,
    result_2: P,
    solutions: IntoIter<Vec<usize>>,
    expanded_count: usize,
    solution_length: usize,
    /// The buffer reused
    expanded_solution: Option<Box<[&'a Move<'id, P>]>>,
    /// The current solution from `solutions` being expanded upon
    currently_expanding_solution: Option<Vec<usize>>,
    /// The state of the canonical sequence expansion
    canonical_sequence_expansion: Option<CanonicalSequenceExpansion>,
    /// The state of the sequence symmetry expansion
    sequence_symmetry_expansion: Option<SequenceSymmetryExpansion>,
}

#[derive(Debug)]
struct SequenceSymmetryExpansion {
    rotation_index: usize,
    rotation_class_size: usize,
}

#[derive(Debug)]
struct CanonicalSequenceExpansion {
    expansion_position: usize,
    expansion_length: usize,
    expansion: Vec<usize>,
}

impl<'id, P: PuzzleState<'id>, T: PruningTables<'id, P>> CycleStructureSolver<'id, P, T> {
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
    /// employ a number of techniques, some specific to a cycle structure solver
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
        mutable: &mut CycleStructureSolverMutable<'id, P, H>,
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
        // and slightly improved by this implementation. Some solution to CCS
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
        let mut move_index_prune_lt = unsafe {
            mutable
                .puzzle_state_history
                .move_index_unchecked(entry_index)
        };
        permitted_cost -= 1;
        // TODO: document this when I am less sleepy
        // skip the "move_index == move_index_prune_pt" case
        // after rotating the end to the front, the move right after (currently
        // the first move) might be lex less than entry_index + 1 (the current
        // move right after)
        // only works on leaf nodes
        if permitted_cost == 0
            // check in bounds
            && entry_index < mutable.puzzle_state_history.stack_pointer()
            // SAFETY: we checked in bounds
            && unsafe { mutable.puzzle_state_history.move_index_unchecked(1) }
                // SAFETY: we checked in bounds
                < unsafe {
                    mutable
                        .puzzle_state_history
                        .move_index_unchecked(entry_index + 1)
                }
        {
            move_index_prune_lt += 1;
        }
        for (move_index, move_) in self
            .puzzle_def
            .moves
            .iter()
            .enumerate()
            // We are at the "X < HISTORY(i)" case described earlier. Pruning
            // all of the child nodes is as simple as skipping a "HISTORY(i)"
            // number of children from the beginning
            .skip(move_index_prune_lt)
        {
            // if self.search_strategy == SearchStrategy::FirstSolution && move_index == 2 && is_root {
            //     return AdmissibleGoalHeuristic::SOLVED;
            // }

            let move_class_index = move_.class_index();
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
            // TODO: investigate optimizing this once CCS becomes more mature
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
                if last_puzzle_state.induces_sorted_cycle_structure(
                    self.pruning_tables.sorted_cycle_structure_ref(),
                    self.puzzle_def.sorted_orbit_defs_ref(),
                    mutable.aux_mem.as_ref_mut(),
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
    /// `CycleStructureSolverError`.
    pub fn solve<H: PuzzleStateHistory<'id, P>>(
        &self,
    ) -> Result<SolutionsIntoIter<'id, '_, P>, CycleStructureSolverError> {
        info!(start!(
            "Beginning Cycle Combination Solver solution search..."
        ));
        let start = Instant::now();

        let mut mutable: CycleStructureSolverMutable<P, H> = CycleStructureSolverMutable {
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
            if last_puzzle_state.induces_sorted_cycle_structure(
                self.pruning_tables.sorted_cycle_structure_ref(),
                self.puzzle_def.sorted_orbit_defs_ref(),
                mutable.aux_mem.as_ref_mut(),
            ) {
                mutable
                    .solutions
                    .push(mutable.puzzle_state_history.create_move_history());
            } else {
                // The loop increments `depth` so we do this manually
                depth = 1;
                if H::GODS_NUMBER.is_some_and(|gods_number| gods_number == 0) {
                    return Err(CycleStructureSolverError::SolutionDoesNotExist);
                }
            }
            debug!(
                working!("Traversed {} nodes in {:.3}s"),
                mutable.nodes_visited,
                depth_start.elapsed().as_secs_f64()
            );
        }

        if !mutable.found_solution() {
            if depth == u8::MAX {
                return Err(CycleStructureSolverError::SolutionDoesNotExist);
            }
            if let Some(max_solution_length) = self.max_solution_length
                && usize::from(depth) > max_solution_length
            {
                return Err(CycleStructureSolverError::MaxSolutionLengthExceeded);
            }
            mutable.nodes_visited = 0;
            mutable
                .puzzle_state_history
                .resize_if_needed(usize::from(depth));
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
                if depth == u8::MAX
                    || H::GODS_NUMBER.is_some_and(|gods_number| usize::from(depth) > gods_number)
                {
                    return Err(CycleStructureSolverError::SolutionDoesNotExist);
                }
                if let Some(max_solution_length) = self.max_solution_length
                    && usize::from(depth) > max_solution_length
                {
                    return Err(CycleStructureSolverError::MaxSolutionLengthExceeded);
                }
                mutable.nodes_visited = 0;
                mutable
                    .puzzle_state_history
                    .resize_if_needed(usize::from(depth));
            }
        }

        info!(
            success!("{} raw solutions at depth {} found in {:.3}s"),
            mutable.solutions.len(),
            depth,
            start.elapsed().as_secs_f64()
        );
        debug!("");
        let result_1 = self.puzzle_def.new_solved_state();
        let result_2 = result_1.clone();
        Ok(SolutionsIntoIter {
            puzzle_def: &self.puzzle_def,
            result_1,
            result_2,
            solutions: mutable.solutions.into_iter(),
            expanded_count: 0,
            solution_length: depth.into(),
            expanded_solution: None,
            currently_expanding_solution: None,
            canonical_sequence_expansion: None,
            sequence_symmetry_expansion: None,
        })
    }
}

impl<'id, P: PuzzleState<'id>> Iterator for SolutionsIntoIter<'id, '_, P> {
    type Item = ();

    // Note that we cannot use a method that mutates self and also returns
    // an Option<&[&Move<'id, P>]>. This would create a mutable borrow that
    // would last while this slice is in use (which is likely in the entire)
    // scope) preventing immutable methods from being called on self.
    // See: https://doc.rust-lang.org/nomicon/lifetime-mismatch.html
    fn next(&mut self) -> Option<()> {
        // The current expansion order:
        //
        // - Canonical sequences
        // - Sequence symmetry

        if self.currently_expanding_solution.is_none() {
            self.currently_expanding_solution = self.solutions.next();
        }
        let currently_expanding_solution = self.currently_expanding_solution.as_deref()?;

        // let old_expansion_info = match &self.canonical_sequence_expansion {
        //     // Some(cse) if cse.expansion.is_some() => None,
        //     Some(cse) => None,
        //     // Some(cse) => Some((cse.expansion_position, cse.expansion_length)),
        //     None => Some((0, 0)),
        // };
        // if let Some(_) = old_expansion_info {
        if self.canonical_sequence_expansion.is_none() {
            for (expansion_position, &move_index_1) in
                currently_expanding_solution.iter().enumerate()
            // .skip(old_expansion_position + old_expansion_length)
            {
                let mut expansion_length = 1;
                let mut acc = Cow::Borrowed(self.puzzle_def.moves[move_index_1].puzzle_state());
                for &move_index_2 in currently_expanding_solution
                    .iter()
                    .skip(expansion_position + 1)
                {
                    self.result_1.replace_compose(
                        &acc,
                        self.puzzle_def.moves[move_index_2].puzzle_state(),
                        self.puzzle_def.sorted_orbit_defs_ref(),
                    );
                    self.result_2.replace_compose(
                        self.puzzle_def.moves[move_index_2].puzzle_state(),
                        &acc,
                        self.puzzle_def.sorted_orbit_defs_ref(),
                    );
                    let commutes = self.result_1 == self.result_2;
                    if !commutes {
                        break;
                    }
                    acc = Cow::Owned(self.result_1.clone());
                    expansion_length += 1;
                }
                if expansion_length != 1 {
                    self.canonical_sequence_expansion = Some(CanonicalSequenceExpansion {
                        expansion_position,
                        expansion_length,
                        expansion: (0..expansion_length).collect_vec(),
                    });
                    break;
                }
            }

            // if let Some(cse) = &self.canonical_sequence_expansion
            //     && cse.expansion.is_none()
            // {
            //     self.canonical_sequence_expansion = None;
            //     self.currently_expanding_solution = None;
            //     return self.next();
            // }
        }

        let reverse_canonical_sequence = |i: usize| {
            let Some(cse) = &self.canonical_sequence_expansion else {
                return i;
            };
            let Some(adjusted) = i.checked_sub(cse.expansion_position) else {
                return i;
            };
            match cse.expansion.get(adjusted) {
                Some(&e) => cse.expansion_position + e,
                None => i,
            }
        };

        let sequence_symmetry_expansion =
            self.sequence_symmetry_expansion.get_or_insert_with(|| {
                let mut rotation_class_size = 1;
                // https://leetcode.com/problems/rotate-string/description/
                while rotation_class_size < self.solution_length
                    && (0..self.solution_length).any(|i| {
                        let reversed_sequence_symmetry =
                            (rotation_class_size + i) % self.solution_length;
                        currently_expanding_solution[reverse_canonical_sequence(i)]
                            != currently_expanding_solution
                                [reverse_canonical_sequence(reversed_sequence_symmetry)]
                    })
                {
                    rotation_class_size += 1;
                }
                SequenceSymmetryExpansion {
                    rotation_index: 0,
                    rotation_class_size,
                }
            });

        let reverse_sequence_symmetry =
            |i: usize| (i + sequence_symmetry_expansion.rotation_index) % self.solution_length;

        if let Some(expanded_solution) = self.expanded_solution.as_deref_mut() {
            for (i, es) in expanded_solution.iter_mut().enumerate() {
                let mut reversified_index = reverse_sequence_symmetry(i);
                reversified_index = reverse_canonical_sequence(reversified_index);
                *es = &self.puzzle_def.moves[currently_expanding_solution[reversified_index]];
            }
        } else {
            self.expanded_solution = Some(
                (0..self.solution_length)
                    .map(|i| {
                        let mut reversified_index = reverse_sequence_symmetry(i);
                        reversified_index = reverse_canonical_sequence(reversified_index);
                        &self.puzzle_def.moves[currently_expanding_solution[reversified_index]]
                    })
                    .collect(),
            );
        }

        sequence_symmetry_expansion.rotation_index += 1;
        if sequence_symmetry_expansion.rotation_index
            // TODO: make this ==
            >= sequence_symmetry_expansion.rotation_class_size
        {
            self.sequence_symmetry_expansion = None;

            if let Some(cse) = &mut self.canonical_sequence_expansion {
                if !pandita1(&mut cse.expansion) {
                    let mut next_cse_exists = false;
                    //
                    for (expansion_position, &move_index_1) in currently_expanding_solution
                        .iter()
                        .enumerate()
                        .skip(cse.expansion_position + cse.expansion_length)
                    {
                        let mut expansion_length = 1;
                        let mut acc =
                            Cow::Borrowed(self.puzzle_def.moves[move_index_1].puzzle_state());
                        for &move_index_2 in currently_expanding_solution
                            .iter()
                            .skip(expansion_position + 1)
                        {
                            self.result_1.replace_compose(
                                &acc,
                                self.puzzle_def.moves[move_index_2].puzzle_state(),
                                self.puzzle_def.sorted_orbit_defs_ref(),
                            );
                            self.result_2.replace_compose(
                                self.puzzle_def.moves[move_index_2].puzzle_state(),
                                &acc,
                                self.puzzle_def.sorted_orbit_defs_ref(),
                            );
                            let commutes = self.result_1 == self.result_2;
                            if !commutes {
                                break;
                            }
                            acc = Cow::Owned(self.result_1.clone());
                            expansion_length += 1;
                        }
                        if expansion_length != 1 {
                            self.canonical_sequence_expansion = Some(CanonicalSequenceExpansion {
                                expansion_position,
                                expansion_length,
                                expansion: (0..expansion_length).collect_vec(),
                            });
                            next_cse_exists = true;
                            break;
                        }
                    }
                    //
                    if !next_cse_exists {
                        self.canonical_sequence_expansion = None;
                        self.currently_expanding_solution = None;
                    }
                }
            } else {
                self.currently_expanding_solution = None;
            }
        }

        self.expanded_count += 1;
        Some(())
    }
}

impl<'id, 'a, P: PuzzleState<'id>> SolutionsIntoIter<'id, 'a, P> {
    /// Returns the expanded solution.
    ///
    /// # Panics
    ///
    /// Panics if this is called before `.next()`
    #[must_use]
    pub fn expanded_solution(&self) -> &[&'a Move<'id, P>] {
        self.expanded_solution.as_ref().unwrap()
    }

    #[must_use]
    pub fn puzzle_def(&self) -> &PuzzleDef<'id, P> {
        self.puzzle_def
    }

    #[must_use]
    pub fn solution_length(&self) -> usize {
        self.solution_length
    }

    #[must_use]
    pub fn expanded_count(&self) -> usize {
        self.expanded_count
    }
}

fn pandita1(perm: &mut [usize]) -> bool {
    let len = perm.len();
    assert!(len > 0);

    let mut i = len - 1;
    while perm[i] < perm[i - 1] {
        if i == 1 {
            return false;
        }
        i -= 1;
    }

    for j in (i..len).rev() {
        if perm[j] > perm[i - 1] {
            perm.swap(i - 1, j);
            break;
        }
    }

    perm[i..].reverse();
    true
}
