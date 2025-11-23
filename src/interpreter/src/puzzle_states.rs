use std::sync::Arc;

use qter_core::{
    I, Int, Program, PuzzleIdx, TheoreticalIdx, U,
    architectures::{Algorithm, Permutation, PermutationGroup},
    discrete_math::{decode, lcm_iter},
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
        self.value += amt % self.order;

        if self.value >= self.order {
            self.value -= self.order;
        }
    }

    pub fn zero_out(&mut self) {
        self.value = Int::zero();
    }

    #[must_use]
    pub fn order(&self) -> Int<U> {
        self.order
    }

    #[must_use]
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

pub trait RobotLike {
    /// Initialize the puzzle in the solved state
    fn initialize(perm_group: Arc<PermutationGroup>) -> Self;

    /// Perform an algorithm on the puzzle
    fn compose_into(&mut self, alg: &Algorithm);

    /// Return the puzzle state as a permutation
    fn take_picture(&self) -> &Permutation;

    /// Solve the puzzle
    fn solve(&mut self);
}

pub trait RobotLikeDyn {
    fn compose_into(&mut self, alg: &Algorithm);

    fn take_picture(&self) -> &Permutation;

    fn solve(&mut self);
}

impl<R: RobotLike> RobotLikeDyn for R {
    fn compose_into(&mut self, alg: &Algorithm) {
        <Self as RobotLike>::compose_into(self, alg);
    }

    fn take_picture(&self) -> &Permutation {
        <Self as RobotLike>::take_picture(self)
    }

    fn solve(&mut self) {
        <Self as RobotLike>::solve(self);
    }
}

pub struct RobotState<R: RobotLike> {
    robot: R,
    perm_group: Arc<PermutationGroup>,
}

impl<R: RobotLike> PuzzleState for RobotState<R> {
    fn compose_into(&mut self, alg: &Algorithm) {
        self.robot.compose_into(alg);
    }

    fn initialize(perm_group: Arc<PermutationGroup>) -> Self {
        RobotState {
            perm_group: Arc::clone(&perm_group),
            robot: R::initialize(perm_group),
        }
    }

    fn facelets_solved(&self, facelets: &[usize]) -> bool {
        let state = self.robot.take_picture();

        for &facelet in facelets {
            let maps_to = state.mapping()[facelet];
            if self.perm_group.facelet_colors()[maps_to]
                != self.perm_group.facelet_colors()[facelet]
            {
                return false;
            }
        }

        true
    }

    fn print(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>> {
        let before = self.robot.take_picture().to_owned();

        let c = self.halt(facelets, generator)?;

        let mut exponentiated = generator.to_owned();
        exponentiated.exponentiate(c.into());

        self.compose_into(&exponentiated);

        if &before != self.robot.take_picture() {
            eprintln!("Printing did not return the cube to the original state!");
            return None;
        }
        Some(c)
    }

    fn halt(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>> {
        let mut generator = generator.to_owned();
        generator.exponentiate(-Int::<U>::one());

        let mut sum = Int::<U>::zero();

        let chromatic_orders = generator.chromatic_orders_by_facelets();
        let order = lcm_iter(facelets.iter().map(|&i| chromatic_orders[i]));

        while !self.facelets_solved(facelets) {
            sum += Int::<U>::one();

            if sum >= order {
                eprintln!(
                    "Decoding failure! Performed as many cycles as the size of the register."
                );
                return None;
            }

            self.compose_into(&generator);
        }

        Some(sum)
    }

    fn repeat_until(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<()> {
        // Halting has the same behavior as repeat_until
        self.halt(facelets, generator).map(|_| ())
    }

    fn solve(&mut self) {
        self.robot.solve();
    }
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
        let mut generator = generator.to_owned();
        generator.exponentiate(-Int::<U>::one());
        let v = decode(&self.state, facelets, &generator)?;
        generator.exponentiate(-v);
        <Self as PuzzleState>::compose_into(self, &generator);
        Some(())
    }
}

impl RobotLike for SimulatedPuzzle {
    fn initialize(perm_group: Arc<PermutationGroup>) -> Self {
        <Self as PuzzleState>::initialize(perm_group)
    }

    fn compose_into(&mut self, alg: &Algorithm) {
        <Self as PuzzleState>::compose_into(self, alg);
    }

    fn take_picture(&self) -> &Permutation {
        self.puzzle_state()
    }

    fn solve(&mut self) {
        <Self as PuzzleState>::solve(self);
    }
}

/// A collection of the states of every puzzle and theoretical register
pub struct PuzzleStates<P: PuzzleState> {
    theoretical_states: Vec<TheoreticalState>,
    puzzle_states: Vec<P>,
}

impl<P: PuzzleState> PuzzleStates<P> {
    #[must_use]
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

    #[must_use]
    pub fn theoretical_state(&self, idx: TheoreticalIdx) -> &TheoreticalState {
        &self.theoretical_states[idx.0]
    }

    #[must_use]
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
