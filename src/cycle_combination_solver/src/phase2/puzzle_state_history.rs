use super::puzzle::{cube3::Cube3, Move, PuzzleDef, PuzzleState};
use std::{marker::PhantomData, ops::Index};

pub trait ValidPuzzleStateHistoryBuf<P: PuzzleState> {}

pub trait PuzzleStateHistoryBuf<P: PuzzleState>:
    ValidPuzzleStateHistoryBuf<P> + Index<usize, Output = P>
{
    fn initialize(puzzle_def: &PuzzleDef<P>) -> Self;
    fn push_stack(&mut self, stack_idx: usize, moove: &Move<P>, puzzle_def: &PuzzleDef<P>);
}

pub struct PuzzleStateHistory<P: PuzzleState, B: PuzzleStateHistoryBuf<P>> {
    stack: B,
    stack_idx: usize,
    _marker: PhantomData<P>,
}

impl<P: PuzzleState, B: PuzzleStateHistoryBuf<P>> From<&PuzzleDef<P>> for PuzzleStateHistory<P, B> {
    fn from(puzzle_def: &PuzzleDef<P>) -> Self {
        Self {
            stack: B::initialize(puzzle_def),
            stack_idx: 0,
            _marker: PhantomData,
        }
    }
}

impl<P: PuzzleState, B: PuzzleStateHistoryBuf<P>> PuzzleStateHistory<P, B> {
    pub fn push_stack(&mut self, moove: &Move<P>, puzzle_def: &PuzzleDef<P>) {
        self.stack.push_stack(self.stack_idx, moove, puzzle_def);
        self.stack_idx += 1;
    }

    pub fn pop_stack(&mut self) {
        self.stack_idx -= 1;
    }
}

impl<P: PuzzleState> ValidPuzzleStateHistoryBuf<P> for Vec<P> {}

impl<P: PuzzleState> PuzzleStateHistoryBuf<P> for Vec<P> {
    fn initialize(puzzle_def: &PuzzleDef<P>) -> Self {
        vec![puzzle_def.solved_state()]
    }

    fn push_stack(&mut self, stack_idx: usize, moove: &Move<P>, puzzle_def: &PuzzleDef<P>) {
        // TODO: move this to the main IDDFS loop!
        if stack_idx + 1 >= self.len() {
            assert!(stack_idx + 1 == self.len()); // Greater than makes no sense

            let mut new_entry = puzzle_def.solved_state();
            new_entry.replace_compose(
                &self[stack_idx],
                &moove.puzzle_state,
                &puzzle_def.sorted_orbit_defs,
            );
            self.push(new_entry);
        } else {
            let (left, right) = self.split_at_mut(stack_idx + 1);
            // SAFETY: At this point of the code, stack_idx + 1 < self.len() must
            // be true, so right is not empty
            let next_entry = unsafe { right.get_unchecked_mut(0) };
            // SAFETY: We split_at_mut at stack_idx + 1, so stack_idx is a valid
            // index
            let last_entry = unsafe { left.get_unchecked(stack_idx) };
            next_entry.replace_compose(
                last_entry,
                &moove.puzzle_state,
                &puzzle_def.sorted_orbit_defs,
            );
        }
    }
}

impl ValidPuzzleStateHistoryBuf<Cube3> for [Cube3; 21] {}

impl<const N: usize, P: PuzzleState> PuzzleStateHistoryBuf<P> for [P; N]
where
    [P; N]: ValidPuzzleStateHistoryBuf<P>,
{
    fn initialize(puzzle_def: &PuzzleDef<P>) -> Self {
        core::array::from_fn(|_| puzzle_def.solved_state())
    }

    fn push_stack(&mut self, stack_idx: usize, moove: &Move<P>, puzzle_def: &PuzzleDef<P>) {
        let (left, right) = self.split_at_mut(stack_idx + 1);
        // SAFETY: ValidPuzzleStackBuf guarantees for the 3x3 that N is 21 as
        // God's number is 20
        let next_entry = unsafe { right.get_unchecked_mut(0) };
        // SAFETY: We split_at_mut at stack_idx + 1, so stack_idx is a valid
        // index
        let last_entry = unsafe { left.get_unchecked(stack_idx) };
        next_entry.replace_compose(
            last_entry,
            &moove.puzzle_state,
            &puzzle_def.sorted_orbit_defs,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::puzzle::cube3::Cube3;
    use puzzle_geometry::ksolve::KPUZZLE_3X3;

    fn puzzle_state_history_composition<B: PuzzleStateHistoryBuf<Cube3>>() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.solved_state();
        let r_move = cube3_def.find_move("R").unwrap();
        let r2_move = cube3_def.find_move("R2").unwrap();
        let mut puzzle_state_history: PuzzleStateHistory<Cube3, B> = (&cube3_def).into();

        puzzle_state_history.push_stack(r_move, &cube3_def);
        puzzle_state_history.push_stack(r_move, &cube3_def);
        assert_eq!(puzzle_state_history.stack_idx, 2);

        let mut r2_state = solved.clone();
        r2_state.replace_compose(&solved, &r2_move.puzzle_state, &cube3_def.sorted_orbit_defs);
        assert_eq!(&puzzle_state_history.stack[2], &r2_state);
    }

    #[test]
    fn test_puzzle_state_history_composition() {
        puzzle_state_history_composition::<Vec<Cube3>>();
        puzzle_state_history_composition::<[Cube3; 21]>();
    }

    fn puzzle_state_history_pop<B: PuzzleStateHistoryBuf<Cube3>>() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.solved_state();
        let r_move = cube3_def.find_move("R").unwrap();
        let r2_move = cube3_def.find_move("R2").unwrap();
        let f2_move = cube3_def.find_move("F2").unwrap();
        let r_prime_move = cube3_def.find_move("R'").unwrap();
        let mut puzzle_state_history: PuzzleStateHistory<Cube3, B> = (&cube3_def).into();

        puzzle_state_history.push_stack(r_move, &cube3_def);
        puzzle_state_history.push_stack(r2_move, &cube3_def);

        assert_eq!(puzzle_state_history.stack_idx, 2);
        let mut r_prime_state = solved.clone();
        r_prime_state.replace_compose(
            &solved,
            &r_prime_move.puzzle_state,
            &cube3_def.sorted_orbit_defs,
        );
        assert_eq!(&puzzle_state_history.stack[2], &r_prime_state);

        puzzle_state_history.pop_stack();

        assert_eq!(puzzle_state_history.stack_idx, 1);
        let mut r_state = solved.clone();
        r_state.replace_compose(&solved, &r_move.puzzle_state, &cube3_def.sorted_orbit_defs);
        assert_eq!(&puzzle_state_history.stack[1], &r_state);

        puzzle_state_history.push_stack(f2_move, &cube3_def);

        assert_eq!(puzzle_state_history.stack_idx, 2);
        let mut r_f2_state = solved.clone();
        r_f2_state.replace_compose(
            &r_state,
            &f2_move.puzzle_state,
            &cube3_def.sorted_orbit_defs,
        );
        assert_eq!(&puzzle_state_history.stack[2], &r_f2_state);

        puzzle_state_history.push_stack(f2_move, &cube3_def);
        puzzle_state_history.push_stack(r_prime_move, &cube3_def);

        assert_eq!(puzzle_state_history.stack_idx, 4);
        assert_eq!(&puzzle_state_history.stack[4], &solved);
    }

    #[test]
    fn test_puzzle_state_history_pop() {
        puzzle_state_history_pop::<Vec<Cube3>>();
        puzzle_state_history_pop::<[Cube3; 21]>();
    }
}
