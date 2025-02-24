use super::puzzle::{cube3::Cube3, Move, PuzzleDef, PuzzleState};
use std::ops::Index;

pub trait PuzzleStateHistoryInterface<P: PuzzleState> {
    // Not the cleanest way of doing this but whatever
    type Buf: PuzzleStateHistoryBuf<P> + Index<usize, Output = (P, usize)>;
}

pub trait PuzzleStateHistoryBuf<P: PuzzleState> {
    fn initialize(puzzle_def: &PuzzleDef<P>) -> Self;
    fn push_stack(&mut self, stack_index: usize, move_index: usize, puzzle_def: &PuzzleDef<P>);
}

pub struct PuzzleStateHistory<P: PuzzleState, B: PuzzleStateHistoryInterface<P>> {
    stack: B::Buf,
    stack_index: usize,
    _marker: std::marker::PhantomData<P>,
}

impl<P: PuzzleState, B: PuzzleStateHistoryInterface<P>> From<&PuzzleDef<P>>
    for PuzzleStateHistory<P, B>
{
    fn from(puzzle_def: &PuzzleDef<P>) -> Self {
        Self {
            stack: B::Buf::initialize(puzzle_def),
            stack_index: 0,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<P: PuzzleState, B: PuzzleStateHistoryInterface<P>> PuzzleStateHistory<P, B> {
    pub fn push_stack(&mut self, move_index: usize, puzzle_def: &PuzzleDef<P>) {
        // B::push_stack(&mut self.stack, self.stack_index, move_index, puzzle_def);
        self.stack
            .push_stack(self.stack_index, move_index, puzzle_def);
        self.stack_index += 1;
    }

    pub fn pop_stack(&mut self) {
        self.stack_index -= 1;
    }

    pub fn last_state(&self) -> &P {
        // TODO: make more stuff unsafe because i am evil
        &self.stack[self.stack_index].0
    }

    pub fn get_move(&self, index: usize) -> usize {
        self.stack[index].1
    }

    pub fn create_move_history(&self, puzzle_def: &PuzzleDef<P>) -> Box<[Move<P>]> {
        let mut move_sequence = Vec::with_capacity(self.stack_index);
        for i in 1..=self.stack_index {
            move_sequence.push(puzzle_def.moves[self.stack[i].1].clone())
        }
        move_sequence.into_boxed_slice()
    }
}

impl<P: PuzzleState> PuzzleStateHistoryInterface<P> for Vec<P> {
    type Buf = Vec<(P, usize)>;
}

impl<P: PuzzleState> PuzzleStateHistoryBuf<P> for Vec<(P, usize)> {
    fn initialize(puzzle_def: &PuzzleDef<P>) -> Vec<(P, usize)> {
        vec![(puzzle_def.solved_state(), 0)]
    }

    fn push_stack(&mut self, stack_index: usize, move_index: usize, puzzle_def: &PuzzleDef<P>) {
        // TODO: unsafe at the end
        let puzzle_state = &puzzle_def.moves[move_index].puzzle_state;
        // TODO: move this to the main IDDFS loop!
        if stack_index + 1 >= self.len() {
            assert!(stack_index + 1 == self.len()); // Greater than makes no sense

            let mut new_entry_puzzle_state = puzzle_def.solved_state();
            new_entry_puzzle_state.replace_compose(
                &self[stack_index].0,
                puzzle_state,
                &puzzle_def.sorted_orbit_defs,
            );
            self.push((new_entry_puzzle_state, move_index));
        } else {
            let (left, right) = self.split_at_mut(stack_index + 1);
            // SAFETY: At this point of the code, stack_index + 1 < self.len() must
            // be true, so right is not empty
            let next_entry = unsafe { right.get_unchecked_mut(0) };
            // SAFETY: We split_at_mut at stack_index + 1, so stack_idx is a valid
            // index
            let last_entry_puzzle_state = unsafe { &left.get_unchecked(stack_index).0 };
            next_entry.0.replace_compose(
                last_entry_puzzle_state,
                puzzle_state,
                &puzzle_def.sorted_orbit_defs,
            );
            next_entry.1 = move_index;
        }
    }
}

pub trait ValidPuzzleStateHistoryBuf<P: PuzzleState> {}

impl ValidPuzzleStateHistoryBuf<Cube3> for [Cube3; 21] {}

impl<const N: usize, P: PuzzleState> PuzzleStateHistoryInterface<P> for [P; N]
where
    [P; N]: ValidPuzzleStateHistoryBuf<P>,
{
    type Buf = [(P, usize); N];
}

impl<const N: usize, P: PuzzleState> PuzzleStateHistoryBuf<P> for [(P, usize); N]
where
    [P; N]: PuzzleStateHistoryInterface<P>,
{
    fn initialize(puzzle_def: &PuzzleDef<P>) -> Self {
        core::array::from_fn(|_| (puzzle_def.solved_state(), 0))
    }

    fn push_stack(&mut self, stack_index: usize, move_index: usize, puzzle_def: &PuzzleDef<P>) {
        let puzzle_state = &puzzle_def.moves[move_index].puzzle_state;
        let (left, right) = self.split_at_mut(stack_index + 1);
        // SAFETY: ValidPuzzleStackBuf guarantees for the 3x3 that N is 21 as
        // God's number is 20
        let next_entry = unsafe { right.get_unchecked_mut(0) };
        // SAFETY: We split_at_mut at stack_index + 1, so stack_idx is a valid
        // index
        let last_entry_puzzle_state = unsafe { &left.get_unchecked(stack_index).0 };
        next_entry.0.replace_compose(
            last_entry_puzzle_state,
            puzzle_state,
            &puzzle_def.sorted_orbit_defs,
        );
        next_entry.1 = move_index;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::puzzle::cube3::Cube3;
    use puzzle_geometry::ksolve::KPUZZLE_3X3;

    fn move_index<P: PuzzleState>(puzzle_def: &PuzzleDef<P>, move_: &Move<P>) -> usize {
        puzzle_def
            .moves
            .iter()
            .position(|move_iter| move_iter.puzzle_state == move_.puzzle_state)
            .unwrap()
    }

    fn puzzle_state_history_composition<B: PuzzleStateHistoryInterface<Cube3>>() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        let r_move_index = move_index(&cube3_def, r_move);
        let mut puzzle_state_history: PuzzleStateHistory<Cube3, B> = (&cube3_def).into();

        puzzle_state_history.push_stack(r_move_index, &cube3_def);
        puzzle_state_history.push_stack(r_move_index, &cube3_def);
        assert_eq!(puzzle_state_history.stack_index, 2);

        let r2_move = cube3_def.find_move("R2").unwrap();
        assert_eq!(&puzzle_state_history.stack[2].0, &r2_move.puzzle_state);
        assert_eq!(puzzle_state_history.stack[1].1, r_move_index);
        assert_eq!(puzzle_state_history.stack[2].1, r_move_index);
    }

    #[test]
    fn test_puzzle_state_history_composition() {
        puzzle_state_history_composition::<Vec<Cube3>>();
        puzzle_state_history_composition::<[Cube3; 21]>();
    }

    fn puzzle_state_history_pop<B: PuzzleStateHistoryInterface<Cube3>>() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.solved_state();
        let r_move = cube3_def.find_move("R").unwrap();
        let r2_move = cube3_def.find_move("R2").unwrap();
        let f2_move = cube3_def.find_move("F2").unwrap();
        let r_prime_move = cube3_def.find_move("R'").unwrap();

        let r_move_index = move_index(&cube3_def, r_move);
        let r2_move_index = move_index(&cube3_def, r2_move);
        let f2_move_index = move_index(&cube3_def, f2_move);
        let r_prime_move_index = move_index(&cube3_def, r_prime_move);

        let mut puzzle_state_history: PuzzleStateHistory<Cube3, B> = (&cube3_def).into();

        puzzle_state_history.push_stack(r_move_index, &cube3_def);
        puzzle_state_history.push_stack(r2_move_index, &cube3_def);

        assert_eq!(puzzle_state_history.stack_index, 2);
        let mut r_prime_state = solved.clone();
        r_prime_state.replace_compose(
            &solved,
            &r_prime_move.puzzle_state,
            &cube3_def.sorted_orbit_defs,
        );
        assert_eq!(&puzzle_state_history.stack[2].0, &r_prime_state);

        puzzle_state_history.pop_stack();

        assert_eq!(puzzle_state_history.stack_index, 1);
        let mut r_state = solved.clone();
        r_state.replace_compose(&solved, &r_move.puzzle_state, &cube3_def.sorted_orbit_defs);
        assert_eq!(&puzzle_state_history.stack[1].0, &r_state);

        puzzle_state_history.push_stack(f2_move_index, &cube3_def);

        assert_eq!(puzzle_state_history.stack_index, 2);
        let mut r_f2_state = solved.clone();
        r_f2_state.replace_compose(
            &r_state,
            &f2_move.puzzle_state,
            &cube3_def.sorted_orbit_defs,
        );
        assert_eq!(&puzzle_state_history.stack[2].0, &r_f2_state);

        puzzle_state_history.push_stack(f2_move_index, &cube3_def);
        puzzle_state_history.push_stack(r_prime_move_index, &cube3_def);

        assert_eq!(puzzle_state_history.stack_index, 4);
        assert_eq!(&puzzle_state_history.stack[4].0, &solved);
    }

    #[test]
    fn test_puzzle_state_history_pop() {
        puzzle_state_history_pop::<Vec<Cube3>>();
        puzzle_state_history_pop::<[Cube3; 21]>();
    }
}
