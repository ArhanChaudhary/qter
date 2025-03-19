//! Canonical canonical_fsm implementation, derived primarily from Lucas Garron's
//! implementation in twsearch with permission:
//! https://github.com/cubing/twsearch/blob/main/src/rs/_internal/canonical_fsm/canonical_fsm.rs

use super::puzzle::{PuzzleDef, PuzzleState};
use std::{collections::HashMap, num::NonZeroUsize};

// Bit N is indexed by a `MoveClassIndex` value of N.
type MoveClassMask = Vec<bool>;

pub type CanonicalFSMState = Option<NonZeroUsize>;

struct MaskToState(HashMap<MoveClassMask, usize>);

struct StateToMask(Vec<MoveClassMask>);

#[derive(Debug)]
pub struct CanonicalFSM<P: PuzzleState> {
    next_state_lookup: Vec<Vec<CanonicalFSMState>>,
    _marker: std::marker::PhantomData<P>,
}

impl<P: PuzzleState> From<&PuzzleDef<P>> for CanonicalFSM<P> {
    fn from(puzzle_def: &PuzzleDef<P>) -> Self {
        let num_move_classes = puzzle_def.move_classes.len();
        let mut commutes: Vec<MoveClassMask> = vec![vec![true; num_move_classes]; num_move_classes];

        // Written this way so if we later iterate over all moves instead of
        // all move classes. This is because multiples can commute differently than their quantum values.
        // For example:
        // - The standard T-Perm (`R U R' U' R' F R2 U' R' U' R U R' F'`) has order 2.
        // - `R2 U2` has order 6.
        // - T-perm and `(R2 U2)3` commute.
        for (i, move_class_1_index) in puzzle_def.move_classes.iter().copied().enumerate() {
            for (j, move_class_2_index) in puzzle_def.move_classes.iter().copied().enumerate() {
                if !puzzle_def.moves[move_class_1_index].commutes_with(
                    &puzzle_def.moves[move_class_2_index],
                    &puzzle_def.sorted_orbit_defs,
                ) {
                    commutes[i][j] = false;
                    commutes[j][i] = false;
                }
            }
        }

        let mut next_state_lookup: Vec<Vec<CanonicalFSMState>> = vec![];

        let mut mask_to_state = MaskToState(HashMap::new());
        mask_to_state.0.insert(vec![false; num_move_classes], 0);
        // state_to_mask, indexed by state ordinal, holds the set of move classes in the
        // move sequence so far for which there has not been a subsequent move that does not
        // commute with that move
        let mut state_to_mask = StateToMask(vec![vec![false; num_move_classes]]);

        // start state
        let mut queue_index = 0;
        while queue_index < state_to_mask.0.len() {
            // illegal state
            let mut next_state = vec![None; num_move_classes];
            let dequeue_move_class_mask = state_to_mask.0[queue_index].clone();

            queue_index += 1;

            for move_class_index in 0..puzzle_def.move_classes.len() {
                let mut skip = false;
                // If there's a greater move (multiple) in the state that
                // commutes with this move's `move_class`, we can't move
                // `move_class`.
                skip |= dequeue_move_class_mask
                    .iter()
                    .zip(commutes[move_class_index].iter())
                    .skip(move_class_index + 1)
                    .any(|(&dequeue_move_class, &commute)| dequeue_move_class && commute);
                skip |= dequeue_move_class_mask[move_class_index];
                if skip {
                    continue;
                }

                let mut next_state_mask = dequeue_move_class_mask.clone();
                for (next_state, commute) in
                    next_state_mask.iter_mut().zip(&commutes[move_class_index])
                {
                    *next_state &= *commute;
                }
                next_state_mask[move_class_index] = true;

                // If a pair of bits are set with the same commutating moves, we
                // can clear out the higher ones. This optimization keeps the
                // state count from going exponential for very big cubes.
                for i in 0..num_move_classes {
                    if next_state_mask[i] {
                        for j in (i + 1)..num_move_classes {
                            if next_state_mask[j] && commutes[i] == commutes[j] {
                                next_state_mask[i] = false;
                            }
                        }
                    }
                }

                next_state[move_class_index] = match mask_to_state.0.get(&next_state_mask) {
                    Some(&state) => CanonicalFSMState::Some(state.try_into().unwrap()),
                    None => {
                        let next_state = state_to_mask.0.len();
                        mask_to_state.0.insert(next_state_mask.clone(), next_state);
                        state_to_mask.0.push(next_state_mask);
                        CanonicalFSMState::Some(next_state.try_into().unwrap())
                    }
                };
            }
            next_state_lookup.push(next_state);
        }

        Self {
            next_state_lookup,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<P: PuzzleState> CanonicalFSM<P> {
    pub fn next_state(
        &self,
        current_fsm_state: CanonicalFSMState,
        move_class_index: usize,
    ) -> CanonicalFSMState {
        // None passed in means we're in the initial state
        // None returned means the move is illegal
        let i = match current_fsm_state {
            Some(state) => state.get(),
            None => 0,
        };
        self.next_state_lookup[i][move_class_index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::puzzle::{cube3::Cube3, HeapPuzzle, PuzzleDef};
    use puzzle_geometry::ksolve::{KPUZZLE_3X3, KPUZZLE_4X4};

    #[test]
    fn test_canonical_fsm_initially_all_legal() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let canonical_fsm: CanonicalFSM<Cube3> = (&cube3_def).into();

        for move_class_index in 0..cube3_def.move_classes.len() {
            assert!(canonical_fsm
                .next_state(CanonicalFSMState::default(), move_class_index)
                .is_some());
        }
    }

    #[test]
    fn test_canonical_fsm_prevents_self() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let canonical_fsm: CanonicalFSM<Cube3> = (&cube3_def).into();
        for move_class_index in 0..cube3_def.move_classes.len() {
            assert!(canonical_fsm
                .next_state(
                    Some(
                        canonical_fsm
                            .next_state(CanonicalFSMState::default(), move_class_index)
                            .unwrap()
                    ),
                    move_class_index
                )
                .is_none());
        }
    }

    #[test]
    fn test_canonical_fsm_prevents_self_and_antipode() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let canonical_fsm: CanonicalFSM<Cube3> = (&cube3_def).into();

        for (move_class_index_1, move_class_1) in cube3_def.move_classes.iter().copied().enumerate()
        {
            let move_1 = &cube3_def.moves[move_class_1];
            for (move_class_index_2, move_class_2) in
                cube3_def.move_classes.iter().copied().enumerate()
            {
                let move_2 = &cube3_def.moves[move_class_2];
                if !move_1.commutes_with(move_2, &cube3_def.sorted_orbit_defs) {
                    continue;
                }

                let allows_1_after_2 = canonical_fsm
                    .next_state(
                        Some(
                            canonical_fsm
                                .next_state(CanonicalFSMState::default(), move_class_index_2)
                                .unwrap(),
                        ),
                        move_class_index_1,
                    )
                    .is_some();
                let allows_2_after_1 = canonical_fsm
                    .next_state(
                        Some(
                            canonical_fsm
                                .next_state(CanonicalFSMState::default(), move_class_index_1)
                                .unwrap(),
                        ),
                        move_class_index_2,
                    )
                    .is_some();

                if move_class_index_1 == move_class_index_2 {
                    // No matter what the same face should not be allowed after
                    // another.
                    assert!(!allows_2_after_1 && !allows_1_after_2);
                } else {
                    // We expect a total ordering of commutative move classes.
                    // Therefore one should be allowed after the other but not
                    // the other way around. Xor gives me that truth table.
                    assert!(allows_1_after_2 ^ allows_2_after_1);
                }
            }
        }
    }

    #[test]
    fn test_big_cube_prevents_move_class() {
        let cube4_def: PuzzleDef<HeapPuzzle> = (&*KPUZZLE_4X4).try_into().unwrap();
        let canonical_fsm: CanonicalFSM<HeapPuzzle> = (&cube4_def).into();

        let mut commutes = vec![];
        for &move_class in &cube4_def.move_classes {
            let mut commute = vec![];
            for (other_move_class_index, &other_move_class) in
                cube4_def.move_classes.iter().enumerate()
            {
                if cube4_def.moves[move_class].commutes_with(
                    &cube4_def.moves[other_move_class],
                    &cube4_def.sorted_orbit_defs,
                ) {
                    commute.push(other_move_class_index);
                }
            }
            commutes.push(commute);
        }
        commutes.dedup();

        for commute in commutes {
            for &move_class_index in &commute {
                for &other_move_class_index in &commute {
                    let current_then_other = canonical_fsm.next_state(
                        Some(
                            canonical_fsm
                                .next_state(CanonicalFSMState::default(), move_class_index)
                                .unwrap(),
                        ),
                        other_move_class_index,
                    );
                    if other_move_class_index <= move_class_index {
                        // a lesser multiple of the move class, not allowed to
                        // move the move class
                        assert!(current_then_other.is_none())
                    } else {
                        // a greater multiple of the move class, allowed to move
                        // the move class
                        assert!(current_then_other.is_some())
                    }
                }
            }
        }
    }

    #[test]
    fn test_big_cube_optimization() {
        let cube4_def: PuzzleDef<HeapPuzzle> = (&*KPUZZLE_4X4).try_into().unwrap();
        let canonical_fsm: CanonicalFSM<HeapPuzzle> = (&cube4_def).into();

        // - 1 to discount the initial FSM state
        assert_eq!(
            canonical_fsm.next_state_lookup.len() - 1,
            canonical_fsm.next_state_lookup[0].len()
        );
    }
}
