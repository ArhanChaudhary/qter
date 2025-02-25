use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::{BitAndAssign, BitOrAssign},
};

use cubing::alg::QuantumMove;

use crate::{
    _internal::{
        errors::SearchError, puzzle_traits::puzzle_traits::SemiGroupActionPuzzle,
        search::indexed_vec::IndexedVec,
    },
    whole_number_newtype,
};

use super::search_generators::{MoveTransformationInfo, SearchGenerators};

const MAX_NUM_MOVE_CLASSES: usize = usize::BITS as usize;

whole_number_newtype!(MoveClassIndex, usize);

// Bit N is indexed by a `MoveClassIndex` value of N.
whole_number_newtype!(MoveClassMask, u64);

impl BitAndAssign for MoveClassMask {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 = self.0 & rhs.0
    }
}

impl BitOrAssign for MoveClassMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 = self.0 | rhs.0
    }
}

whole_number_newtype!(CanonicalFSMState, usize);

pub(crate) const CANONICAL_FSM_START_STATE: CanonicalFSMState = CanonicalFSMState(0);
pub(crate) const ILLEGAL_FSM_STATE: CanonicalFSMState = CanonicalFSMState(0xFFFFFFFF);

#[derive(Default, Debug)]
struct MaskToState(HashMap<MoveClassMask, CanonicalFSMState>);

impl MaskToState {
    pub fn insert(&mut self, mask: MoveClassMask, state: CanonicalFSMState) {
        self.0.insert(mask, state);
    }

    pub fn get(&self, mask: MoveClassMask) -> Option<CanonicalFSMState> {
        self.0.get(&mask).copied() // TODO: figure out how to do this safely and most performantly
    }
}

#[derive(Default, Debug)]
struct StateToMask(IndexedVec<CanonicalFSMState, MoveClassMask>);

impl StateToMask {
    pub fn new(initial_value: MoveClassMask) -> StateToMask {
        Self(IndexedVec::new(vec![initial_value]))
    }

    // Push the next value (useful while constructing in order.)
    pub fn push(&mut self, mask: MoveClassMask) {
        self.0.push(mask);
    }

    pub fn set(&mut self, state: CanonicalFSMState, value: MoveClassMask) {
        self.0.set(state, value);
    }

    pub fn get(&self, state: CanonicalFSMState) -> MoveClassMask {
        *self.0.at(state)
    }
}

#[derive(Debug)]
pub struct CanonicalFSM<TPuzzle: SemiGroupActionPuzzle> {
    // disallowed_move_classes, indexed by state ordinal, holds the set of move classes that should
    // not be made from this state.
    // disallowed_move_classes: StateToMask,
    pub(crate) next_state_lookup:
        IndexedVec<CanonicalFSMState, IndexedVec<MoveClassIndex, CanonicalFSMState>>,

    phantom_data: PhantomData<TPuzzle>,
}

#[derive(Debug, Default)]
pub struct CanonicalFSMConstructionOptions {
    pub forbid_transitions_by_quantums_either_direction: HashSet<(QuantumMove, QuantumMove)>,
}

impl CanonicalFSMConstructionOptions {
    fn is_transition_forbidden<TPuzzle: SemiGroupActionPuzzle>(
        &self,
        move1_info: &MoveTransformationInfo<TPuzzle>,
        move2_info: &MoveTransformationInfo<TPuzzle>,
    ) -> bool {
        self.forbid_transitions_by_quantums_either_direction
            .contains(&(
                move1_info.r#move.quantum.as_ref().clone(),
                move2_info.r#move.quantum.as_ref().clone(),
            ))
            || self
                .forbid_transitions_by_quantums_either_direction
                .contains(&(
                    move2_info.r#move.quantum.as_ref().clone(),
                    move1_info.r#move.quantum.as_ref().clone(),
                ))
    }
}

impl<TPuzzle: SemiGroupActionPuzzle> CanonicalFSM<TPuzzle> {
    // TODO: Return a more specific error.
    /// Pass `Default::default()` as for `options` when no options are needed.
    pub fn try_new(
        tpuzzle: TPuzzle,
        generators: SearchGenerators<TPuzzle>, // TODO: make this a field in the options?
        options: CanonicalFSMConstructionOptions,
    ) -> Result<CanonicalFSM<TPuzzle>, SearchError> {
        let num_move_classes = generators.by_move_class.len();
        if num_move_classes > MAX_NUM_MOVE_CLASSES {
            return Err(SearchError {
                description: "Too many move classes!".to_owned(),
            });
        }

        let mut commutes: Vec<MoveClassMask> =
            vec![MoveClassMask((1 << num_move_classes) - 1); num_move_classes];
        let mut forbidden_transitions: Vec<MoveClassMask> =
            vec![MoveClassMask(0); num_move_classes];

        // Written this way so if we later iterate over all moves instead of
        // all move classes. This is because multiples can commute differently than their quantum values.
        // For example:
        // - The standard T-Perm (`R U R' U' R' F R2 U' R' U' R U R' F'`) has order 2.
        // - `R2 U2` has order 6.
        // - T-perm and `(R2 U2)3` commute.
        for i in 0..num_move_classes {
            let i = MoveClassIndex(i);
            for j in 0..num_move_classes {
                let j = MoveClassIndex(j);
                let move1_info = &generators.by_move_class.at(i)[0];
                let move2_info = &generators.by_move_class.at(j)[0];
                if !tpuzzle.do_moves_commute(move1_info, move2_info) {
                    commutes[*i] &= MoveClassMask(!(1 << *j));
                    commutes[*j] &= MoveClassMask(!(1 << *i));
                }
                if options.is_transition_forbidden(move1_info, move2_info) {
                    forbidden_transitions[*i] |= MoveClassMask(1 << *j);
                    forbidden_transitions[*j] |= MoveClassMask(1 << *i);
                }
            }
        }

        let mut next_state_lookup: IndexedVec<
            CanonicalFSMState,
            IndexedVec<MoveClassIndex, CanonicalFSMState>,
        > = IndexedVec::default();

        let mut mask_to_state = MaskToState::default();
        mask_to_state.insert(MoveClassMask(0), CANONICAL_FSM_START_STATE);
        let mut state_to_mask = StateToMask::new(MoveClassMask(0));
        // state_to_mask, indexed by state ordinal,  holds the set of move classes in the
        // move sequence so far for which there has not been a subsequent move that does not
        // commute with that move.
        let mut disallowed_move_classes = StateToMask::new(MoveClassMask(0));

        let mut queue_index: CanonicalFSMState = CANONICAL_FSM_START_STATE;
        while Into::<usize>::into(queue_index) < state_to_mask.0.len() {
            let mut next_state: IndexedVec<MoveClassIndex, CanonicalFSMState> =
                IndexedVec::new(vec![ILLEGAL_FSM_STATE; num_move_classes]);

            let dequeue_move_class_mask: MoveClassMask = state_to_mask.get(queue_index);
            disallowed_move_classes.push(MoveClassMask(0));

            queue_index += CanonicalFSMState(1);
            let from_state = queue_index;

            for move_class_index in generators.by_move_class.index_iter() {
                let mut skip = false;
                // If there's a greater move (multiple) in the state that
                // commutes with this move's `move_class`, we can't move
                // `move_class`.
                skip |= (*dequeue_move_class_mask & *commutes[*move_class_index])
                    >> (*move_class_index + 1)
                    != 0;
                skip |= (*dequeue_move_class_mask & *forbidden_transitions[*move_class_index]) != 0;
                skip |= ((*dequeue_move_class_mask >> *move_class_index) & 1) != 0;
                if skip {
                    let new_value = MoveClassMask(
                        *disallowed_move_classes.get(from_state) | (1 << *move_class_index),
                    );
                    disallowed_move_classes.set(from_state, new_value);
                    continue;
                }
                // TODO implement bit arithmetic for whole number newtypes.
                let mut next_state_bits = (*dequeue_move_class_mask & *commutes[*move_class_index])
                    | (1 << *move_class_index);
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

                next_state.set(
                    move_class_index,
                    match mask_to_state.get(next_move_mask_class) {
                        None => {
                            let next_state = CanonicalFSMState(state_to_mask.0.len());
                            mask_to_state.insert(next_move_mask_class, next_state);
                            state_to_mask.push(next_move_mask_class);
                            next_state
                        }
                        Some(state) => state,
                    },
                );
            }
            next_state_lookup.push(next_state);
        }

        Ok(Self {
            next_state_lookup,
            phantom_data: PhantomData,
        })
    }

    pub(crate) fn next_state(
        &self,
        current_fsm_state: CanonicalFSMState,
        move_class_index: MoveClassIndex,
    ) -> Option<CanonicalFSMState> {
        match *self
            .next_state_lookup
            .at(current_fsm_state)
            .at(move_class_index)
        {
            ILLEGAL_FSM_STATE => None,
            state => Some(state),
        }
    }
}