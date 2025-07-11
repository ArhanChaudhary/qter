use std::sync::Arc;

use qter_core::{
    I, Int, Program, PuzzleIdx, TheoreticalIdx, U,
    architectures::{Algorithm, Permutation, PermutationGroup},
    discrete_math::decode,
};

/// An instance of a theoretical register. Analagous to the `Puzzle` structure.
pub struct TheoreticalState {
    value: Int<U>,
    order: Int<U>,
}

impl TheoreticalState {
    pub fn add_to_i(&mut self, amt: Int<I>) {
        self.add_to(amt % self.order);
    }

    pub fn add_to(&mut self, amt: Int<U>) {
        assert!(amt < self.order);

        self.value += amt % self.order;

        if self.value >= self.order {
            self.value -= self.order;
        }
    }

    pub fn zero_out(&mut self) {
        self.value = Int::zero();
    }

    pub fn order(&self) -> Int<U> {
        self.order
    }

    pub fn value(&self) -> Int<U> {
        self.value
    }
}

pub trait PuzzleState {
    /// Initialize the `Puzzle` in the solved state
    fn initialize(perm_group: Arc<PermutationGroup>) -> Self;

    /// Perform an algorithm on the puzzle state
    fn compose_into(&mut self, alg: &Algorithm);

    /// Check whether the given facelets are solved
    fn facelets_solved(&self, facelets: &[usize]) -> bool;

    /// Decode the permutation using the register generator and the given facelets.
    ///
    /// In general, an arbitrary scramble cannot be decoded. If this is the case, the function will return `None`.
    ///
    /// This function should not alter the cube state unless it returns `None`.
    fn print(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>>;

    /// Decode the register without requiring the cube state to be unaltered.
    fn halt(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>> {
        self.print(facelets, generator)
    }

    /// Repeat the algorithm until the given facelets are solved.
    ///
    /// Returns None if the facelets cannot be solved by repeating the algorithm.
    fn repeat_until(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<()>;

    /// Bring the puzzle to the solved state
    fn solve(&mut self);
}

#[derive(Clone, Debug)]
pub struct SimulatedPuzzle {
    perm_group: Arc<PermutationGroup>,
    pub(crate) state: Permutation,
}

impl SimulatedPuzzle {
    /// Get the state underlying the puzzle
    pub fn puzzle_state(&self) -> &Permutation {
        &self.state
    }
}

impl PuzzleState for SimulatedPuzzle {
    fn initialize(perm_group: Arc<PermutationGroup>) -> Self {
        SimulatedPuzzle {
            state: perm_group.identity(),
            perm_group,
        }
    }

    fn compose_into(&mut self, alg: &Algorithm) {
        self.state.compose_into(alg.permutation());
    }

    fn facelets_solved(&self, facelets: &[usize]) -> bool {
        for &facelet in facelets {
            let maps_to = self.state.mapping()[facelet];
            if self.perm_group.facelet_colors()[maps_to]
                != self.perm_group.facelet_colors()[facelet]
            {
                return false;
            }
        }

        true
    }

    fn print(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>> {
        decode(&self.state, facelets, generator)
    }

    fn solve(&mut self) {
        self.state = self.perm_group.identity();
    }

    fn repeat_until(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<()> {
        let v = decode(&self.state, facelets, generator)?;
        let mut generator = generator.to_owned();
        generator.exponentiate(v.into());
        self.compose_into(&generator);
        Some(())
    }
}

/// A collection of the states of every puzzle and theoretical register
pub struct PuzzleStates<P: PuzzleState> {
    theoretical_states: Vec<TheoreticalState>,
    puzzle_states: Vec<P>,
}

impl<P: PuzzleState> PuzzleStates<P> {
    pub fn new(program: &Program) -> Self {
        let theoretical_states = program
            .theoretical
            .iter()
            .map(|order| TheoreticalState {
                value: Int::zero(),
                order: **order,
            })
            .collect();

        let puzzle_states = program
            .puzzles
            .iter()
            .map(|perm_group| P::initialize(Arc::clone(perm_group)))
            .collect();

        PuzzleStates {
            theoretical_states,
            puzzle_states,
        }
    }

    pub fn theoretical_state(&self, idx: TheoreticalIdx) -> &TheoreticalState {
        &self.theoretical_states[idx.0]
    }

    pub fn puzzle_state(&self, idx: PuzzleIdx) -> &P {
        &self.puzzle_states[idx.0]
    }

    pub fn theoretical_state_mut(&mut self, idx: TheoreticalIdx) -> &mut TheoreticalState {
        &mut self.theoretical_states[idx.0]
    }

    pub fn puzzle_state_mut(&mut self, idx: PuzzleIdx) -> &mut P {
        &mut self.puzzle_states[idx.0]
    }
}
