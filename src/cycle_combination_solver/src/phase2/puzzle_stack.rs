use super::puzzle::{cube3::Cube3, KSolveConversionError, Move, PuzzleDef, PuzzleState};
use std::{marker::PhantomData, ops::Index};

pub trait ValidPuzzleStackBuf<P: PuzzleState> {}

pub trait PuzzleStackBuf<P: PuzzleState>:
    ValidPuzzleStackBuf<P> + Index<usize, Output = P> + Sized
{
    fn initialize(puzzle_def: &PuzzleDef<P>) -> Result<Self, KSolveConversionError>;
    fn push_stack(&mut self, stack_idx: usize, moove: &Move<P>, puzzle_def: &PuzzleDef<P>);
}

pub struct PuzzleStack<P: PuzzleState, B: PuzzleStackBuf<P>> {
    stack: B,
    stack_idx: usize,
    _marker: PhantomData<P>,
}

impl<P: PuzzleState, B: PuzzleStackBuf<P>> TryFrom<&PuzzleDef<P>> for PuzzleStack<P, B> {
    type Error = KSolveConversionError;

    fn try_from(puzzle_def: &PuzzleDef<P>) -> Result<Self, KSolveConversionError> {
        Ok(Self {
            stack: B::initialize(puzzle_def)?,
            stack_idx: 0,
            _marker: PhantomData,
        })
    }
}

impl<P: PuzzleState, B: PuzzleStackBuf<P>> PuzzleStack<P, B> {
    pub fn push_stack(&mut self, moove: &Move<P>, puzzle_def: &PuzzleDef<P>) {
        self.stack.push_stack(self.stack_idx, moove, puzzle_def);
        self.stack_idx += 1;
    }

    pub fn pop_stack(&mut self) {
        self.stack_idx -= 1;
    }
}

impl<P: PuzzleState> ValidPuzzleStackBuf<P> for Vec<P> {}

impl<P: PuzzleState> PuzzleStackBuf<P> for Vec<P> {
    fn initialize(puzzle_def: &PuzzleDef<P>) -> Result<Self, KSolveConversionError> {
        Ok(vec![puzzle_def.solved_state()?])
    }

    fn push_stack(&mut self, stack_idx: usize, moove: &Move<P>, puzzle_def: &PuzzleDef<P>) {
        if stack_idx + 1 >= self.len() {
            assert!(stack_idx + 1 == self.len()); // Greater than makes no sense
                                                  // We can unwrap because initialize had to have ran before and call
                                                  // solved_state
            let mut new_entry = puzzle_def.solved_state().unwrap();
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

impl ValidPuzzleStackBuf<Cube3> for [Cube3; 21] {}

impl<const N: usize, P: PuzzleState> PuzzleStackBuf<P> for [P; N]
where
    [P; N]: ValidPuzzleStackBuf<P>,
{
    fn initialize(puzzle_def: &PuzzleDef<P>) -> Result<Self, KSolveConversionError> {
        let solved = puzzle_def.solved_state()?;
        Ok(core::array::from_fn(|_| solved.clone()))
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

    fn puzzle_stack_composition<B: PuzzleStackBuf<Cube3>>() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.solved_state().unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        let r2_move = cube3_def.find_move("R2").unwrap();
        let mut puzzle_stack: PuzzleStack<Cube3, B> = (&cube3_def).try_into().unwrap();

        puzzle_stack.push_stack(r_move, &cube3_def);
        puzzle_stack.push_stack(r_move, &cube3_def);
        assert_eq!(puzzle_stack.stack_idx, 2);

        let mut r2_state = solved.clone();
        r2_state.replace_compose(&solved, &r2_move.puzzle_state, &cube3_def.sorted_orbit_defs);
        assert_eq!(&puzzle_stack.stack[2], &r2_state);
    }

    #[test]
    fn test_puzzle_stack_composition() {
        puzzle_stack_composition::<Vec<Cube3>>();
        puzzle_stack_composition::<[Cube3; 21]>();
    }

    fn puzzle_stack_pop<B: PuzzleStackBuf<Cube3>>() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.solved_state().unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        let r2_move = cube3_def.find_move("R2").unwrap();
        let f2_move = cube3_def.find_move("F2").unwrap();
        let r_prime_move = cube3_def.find_move("R'").unwrap();
        let mut puzzle_stack: PuzzleStack<Cube3, B> = (&cube3_def).try_into().unwrap();

        puzzle_stack.push_stack(r_move, &cube3_def);
        puzzle_stack.push_stack(r2_move, &cube3_def);

        assert_eq!(puzzle_stack.stack_idx, 2);
        let mut r_prime_state = solved.clone();
        r_prime_state.replace_compose(
            &solved,
            &r_prime_move.puzzle_state,
            &cube3_def.sorted_orbit_defs,
        );
        assert_eq!(&puzzle_stack.stack[2], &r_prime_state);

        puzzle_stack.pop_stack();

        assert_eq!(puzzle_stack.stack_idx, 1);
        let mut r_state = solved.clone();
        r_state.replace_compose(&solved, &r_move.puzzle_state, &cube3_def.sorted_orbit_defs);
        assert_eq!(&puzzle_stack.stack[1], &r_state);

        puzzle_stack.push_stack(f2_move, &cube3_def);

        assert_eq!(puzzle_stack.stack_idx, 2);
        let mut r_f2_state = solved.clone();
        r_f2_state.replace_compose(
            &r_state,
            &f2_move.puzzle_state,
            &cube3_def.sorted_orbit_defs,
        );
        assert_eq!(&puzzle_stack.stack[2], &r_f2_state);

        puzzle_stack.push_stack(f2_move, &cube3_def);
        puzzle_stack.push_stack(r_prime_move, &cube3_def);

        assert_eq!(puzzle_stack.stack_idx, 4);
        assert_eq!(&puzzle_stack.stack[4], &solved);
    }

    #[test]
    fn test_puzzle_stack_pop() {
        puzzle_stack_pop::<Vec<Cube3>>();
        puzzle_stack_pop::<[Cube3; 21]>();
    }
}
