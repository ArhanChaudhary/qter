use super::puzzle::{KSolveConversionError, PuzzleDef, PuzzleState};
use std::collections::HashMap;

const MAX_NUM_MOVE_CLASSES: usize = usize::BITS as usize;

// Bit N is indexed by a `MoveClassIndex` value of N.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
struct MoveClassMask(u64);

#[derive(Clone, Copy, Debug)]
pub struct CanonicalFSMState(usize);

struct MaskToState(HashMap<MoveClassMask, CanonicalFSMState>);

struct StateToMask(Vec<MoveClassMask>);

#[derive(Debug)]
pub struct CanonicalFSM<P: PuzzleState> {
    pub next_state_lookup: Vec<Vec<CanonicalFSMState>>,
    _marker: std::marker::PhantomData<P>,
}

impl<P: PuzzleState> TryFrom<PuzzleDef<P>> for CanonicalFSM<P> {
    type Error = KSolveConversionError;

    fn try_from(puzzle_def: PuzzleDef<P>) -> Result<Self, Self::Error> {
        let num_move_classes = puzzle_def.class_moves.len();
        if num_move_classes > MAX_NUM_MOVE_CLASSES {
            return Err(KSolveConversionError::TooManyMoveClasses);
        }

        let mut commutes = vec![MoveClassMask((1 << num_move_classes) - 1); num_move_classes];

        // Written this way so if we later iterate over all moves instead of
        // all move classes. This is because multiples can commute differently than their quantum values.
        // For example:
        // - The standard T-Perm (`R U R' U' R' F R2 U' R' U' R U R' F'`) has order 2.
        // - `R2 U2` has order 6.
        // - T-perm and `(R2 U2)3` commute.
        for (i, move_class_1) in puzzle_def.class_moves.iter().enumerate() {
            for (j, move_class_2) in puzzle_def.class_moves.iter().enumerate() {
                if !move_class_1.commutes_with(move_class_2, &puzzle_def.sorted_orbit_defs) {
                    commutes[i].0 &= !(1 << j);
                    commutes[j].0 &= !(1 << i);
                }
            }
        }

        let mut next_state_lookup = vec![];

        let mut mask_to_state = MaskToState(HashMap::new());
        mask_to_state
            .0
            .insert(MoveClassMask(0), CanonicalFSMState(0));
        let mut state_to_mask = StateToMask(vec![MoveClassMask(0)]);
        // state_to_mask, indexed by state ordinal,  holds the set of move classes in the
        // move sequence so far for which there has not been a subsequent move that does not
        // commute with that move
        let mut disallowed_move_classes = StateToMask(vec![MoveClassMask(0)]);

        // start state
        let mut queue_index = CanonicalFSMState(0);
        while queue_index.0 < state_to_mask.0.len() {
            // illegal state
            let mut next_state = vec![CanonicalFSMState(0xFFFFFFFF); num_move_classes];

            let dequeue_move_class_mask = state_to_mask.0[queue_index.0];
            disallowed_move_classes.0.push(MoveClassMask(0));

            queue_index.0 += 1;
            let from_state = queue_index;

            for move_class_index in 0..puzzle_def.class_moves.len() {
                let mut skip = false;
                // If there's a greater move (multiple) in the state that
                // commutes with this move's `move_class`, we can't move
                // `move_class`.
                skip |= (dequeue_move_class_mask.0 & commutes[move_class_index].0)
                    >> (move_class_index + 1)
                    != 0;
                skip |= ((dequeue_move_class_mask.0 >> move_class_index) & 1) != 0;
                if skip {
                    let new_value = MoveClassMask(
                        disallowed_move_classes.0[from_state.0].0 | (1 << move_class_index),
                    );
                    disallowed_move_classes.0[from_state.0] = new_value;
                    continue;
                }

                let mut next_state_bits = (dequeue_move_class_mask.0
                    & commutes[move_class_index].0)
                    | (1 << move_class_index);
                // If a pair of bits are set with the same commutating moves, we
                // can clear out the higher ones. This optimization keeps the
                // state count from going exponential for very big cubes.
                for i in 0..num_move_classes {
                    if (next_state_bits >> i) & 1 != 0 {
                        for j in (i + 1)..num_move_classes {
                            if ((next_state_bits >> j) & 1) != 0 && commutes[i] == commutes[j] {
                                next_state_bits &= !(1 << i);
                            }
                        }
                    }
                }

                let next_move_mask_class = MoveClassMask(next_state_bits);
                next_state[move_class_index] = match mask_to_state.0.get(&next_move_mask_class) {
                    Some(&state) => state,
                    None => {
                        let next_state = CanonicalFSMState(state_to_mask.0.len());
                        mask_to_state.0.insert(next_move_mask_class, next_state);
                        state_to_mask.0.push(next_move_mask_class);
                        next_state
                    }
                };
            }
            next_state_lookup.push(next_state);
        }
        Ok(Self {
            next_state_lookup,
            _marker: std::marker::PhantomData,
        })
    }
}

impl<P: PuzzleState> CanonicalFSM<P> {
    pub fn next_state(
        &self,
        current_fsm_state: CanonicalFSMState,
        move_class_index: usize,
    ) -> Option<CanonicalFSMState> {
        match self.next_state_lookup[current_fsm_state.0][move_class_index] {
            CanonicalFSMState(0xFFFFFFFF) => None,
            state => Some(state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::puzzle::{cube3::Cube3, PuzzleDef};
    use puzzle_geometry::ksolve::KPUZZLE_3X3;

    #[test]
    fn test_thing() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let fsm: CanonicalFSM<Cube3> = cube3_def.try_into().unwrap();
        for (i, state) in fsm.next_state_lookup.iter().enumerate() {
            println!("State {i}");
            for (j, next_state) in state.iter().enumerate() {
                println!("Move class {j}: {:032b}", next_state.0);
            }
        }
    }
}
