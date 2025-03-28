use num_traits::PrimInt;
use puzzle_geometry::ksolve::KSolve;
use std::hash::Hash;
use std::{fmt::Debug, num::NonZeroU8};
use thiserror::Error;

pub mod cube3;

pub trait PuzzleState: Hash + Clone + PartialEq + Debug {
    type MultiBv: MultiBvInterface;

    /// Get a default multi bit vector for use in `induces_sorted_cycle_type`
    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv;
    /// Try to create a puzzle state from a sorted transformation and sorted
    /// orbit defs, checking if a puzzle state can be created from the orbit
    /// defs. `sorted_transformations` is guaranteed to correspond to
    /// `sorted_orbit_defs`.
    fn try_from_transformation_meta(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError>;
    /// Compose two puzzle states in place
    fn replace_compose(&mut self, a: &Self, b: &Self, sorted_orbit_defs: &[OrbitDef]);
    /// Inverse of a puzzle state
    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: &[OrbitDef]);
    /// The goal state for IDA* search
    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: <Self::MultiBv as MultiBvInterface>::MultiBvReusableRef<'_>,
    ) -> bool;
    /// Get the bytes of the specified orbit index in the form (permutation
    /// vector, orientation vector).
    fn orbit_bytes_by_index(&self, index: usize, sorted_orbit_defs: &[OrbitDef]) -> (&[u8], &[u8]);
}

pub trait MultiBvInterface {
    type MultiBvReusableRef<'a>
    where
        Self: 'a;

    fn reusable_ref(&mut self) -> Self::MultiBvReusableRef<'_>;
}

impl MultiBvInterface for Box<[u8]> {
    type MultiBvReusableRef<'a> = &'a mut [u8];

    fn reusable_ref(&mut self) -> Self::MultiBvReusableRef<'_> {
        self
    }
}

impl<T: PrimInt, const N: usize> MultiBvInterface for [T; N] {
    type MultiBvReusableRef<'a>
        = [T; N]
    where
        T: 'a;

    fn reusable_ref(&mut self) -> Self::MultiBvReusableRef<'_> {
        *self
    }
}

impl MultiBvInterface for () {
    type MultiBvReusableRef<'a> = ();

    fn reusable_ref(&mut self) -> Self::MultiBvReusableRef<'_> {}
}

#[derive(Clone, PartialEq, Debug, Hash)]
pub struct StackPuzzle<const N: usize>([u8; N]);

#[derive(Clone, PartialEq, Debug, Hash)]
pub struct HeapPuzzle(Box<[u8]>);

#[derive(Debug)]
pub struct PuzzleDef<P: PuzzleState> {
    pub moves: Box<[Move<P>]>,
    // indicies into moves
    pub move_classes: Box<[usize]>,
    pub symmetries: Box<[Move<P>]>,
    pub sorted_orbit_defs: Box<[OrbitDef]>,
    pub name: String,
}

#[derive(Error, Debug)]
pub enum KSolveConversionError {
    #[error(
        "Phase 2 does not currently support puzzles with set sizes larger than 255, but it will in the future"
    )]
    SetSizeTooBig,
    #[error("Not enough buffer space to convert move")]
    NotEnoughBufferSpace,
    #[error("Could not expand move set, order of a move too high")]
    MoveOrderTooHigh,
    #[error("Too many move classes")]
    TooManyMoveClasses,
    #[error("Invalid KSolve orbit definitions. Expected: {0:?}\nActual: {1:?}")]
    InvalidOrbitDefs(Vec<OrbitDef>, Vec<OrbitDef>),
}

#[derive(Debug, Clone)]
pub struct Move<P: PuzzleState> {
    pub puzzle_state: P,
    pub move_class_index: usize,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct OrbitDef {
    pub piece_count: NonZeroU8,
    pub orientation_count: NonZeroU8,
}

pub type OrientedPartition = Vec<(NonZeroU8, bool)>;

impl<P: PuzzleState> Move<P> {
    pub fn commutes_with(
        &self,
        other: &Self,
        result_1: &mut P,
        result_2: &mut P,
        sorted_orbit_defs: &[OrbitDef],
    ) -> bool {
        result_1.replace_compose(&self.puzzle_state, &other.puzzle_state, sorted_orbit_defs);
        result_2.replace_compose(&other.puzzle_state, &self.puzzle_state, sorted_orbit_defs);
        result_1 == result_2
    }
}

impl<P: PuzzleState> PuzzleDef<P> {
    pub fn find_move(&self, name: &str) -> Option<&Move<P>> {
        self.moves.iter().find(|move_| move_.name == name)
    }

    pub fn find_symmetry(&self, name: &str) -> Option<&Move<P>> {
        self.symmetries.iter().find(|move_| move_.name == name)
    }

    pub fn new_solved_state(&self) -> P {
        solved_state_from_sorted_orbit_defs(&self.sorted_orbit_defs)
    }
}

fn solved_state_from_sorted_orbit_defs<P: PuzzleState>(sorted_orbit_defs: &[OrbitDef]) -> P {
    let sorted_transformations = sorted_orbit_defs
        .iter()
        .map(|orbit_def| {
            (0..orbit_def.piece_count.get())
                .map(|i| (i, 0))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    // We can unwrap because try_from guarantees that the orbit defs are valid
    P::try_from_transformation_meta(&sorted_transformations, sorted_orbit_defs).unwrap()
}

impl<P: PuzzleState> TryFrom<&KSolve> for PuzzleDef<P> {
    type Error = KSolveConversionError;

    fn try_from(ksolve: &KSolve) -> Result<Self, Self::Error> {
        let mut sorted_orbit_defs: Vec<OrbitDef> = ksolve
            .sets()
            .iter()
            .map(|ksolve_set| {
                Ok(OrbitDef {
                    piece_count: ksolve_set
                        .piece_count()
                        .try_into()
                        .map_err(|_| KSolveConversionError::SetSizeTooBig)?,
                    orientation_count: ksolve_set.orientation_count(),
                })
            })
            .collect::<Result<_, KSolveConversionError>>()?;

        let mut arg_indicies = (0..sorted_orbit_defs.len()).collect::<Vec<_>>();
        arg_indicies.sort_by_key(|&i| {
            (
                sorted_orbit_defs[i].piece_count.get(),
                sorted_orbit_defs[i].orientation_count.get(),
            )
        });

        sorted_orbit_defs = arg_indicies
            .iter()
            .map(|&i| sorted_orbit_defs[i].clone())
            .collect();

        let mut moves = Vec::with_capacity(ksolve.moves().len());
        let mut move_classes = vec![];
        let mut symmetries = Vec::with_capacity(ksolve.symmetries().len());

        for (i, ksolve_move) in ksolve
            .moves()
            .iter()
            .chain(ksolve.symmetries().iter())
            .enumerate()
        {
            let mut sorted_transformations = ksolve_move
                .transformation()
                .iter()
                .map(|perm_and_ori| {
                    perm_and_ori
                        .iter()
                        .map(|&(perm, orientation)| {
                            // we can unwrap because sorted_orbit_defs exists
                            ((perm.get() - 1).try_into().unwrap(), orientation)
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            sorted_transformations = arg_indicies
                .iter()
                .map(|&i| sorted_transformations[i].clone())
                .collect();

            let puzzle_state = P::try_from_transformation_meta(
                &sorted_transformations,
                sorted_orbit_defs.as_slice(),
            )?;

            if i >= ksolve.moves().len() {
                let base_move = Move {
                    name: ksolve_move.name().to_owned(),
                    move_class_index: 0,
                    puzzle_state,
                };
                symmetries.push(base_move);
                continue;
            }

            let mut result_1 = puzzle_state.clone();
            let mut result_2 = puzzle_state.clone();

            let move_class = moves.len();
            let move_class_index = move_classes.len();
            let base_move = Move {
                name: ksolve_move.name().to_owned(),
                move_class_index,
                puzzle_state,
            };

            let solved: P = solved_state_from_sorted_orbit_defs(&sorted_orbit_defs);

            let base_name = base_move.name.clone();
            move_classes.push(move_class);

            let mut move_powers: Vec<P> = vec![];
            const MAX_MOVE_POWER: usize = 1_000_000;

            for _ in 0..MAX_MOVE_POWER {
                result_1.replace_compose(&result_2, &base_move.puzzle_state, &sorted_orbit_defs);
                if result_1 == solved {
                    break;
                }
                move_powers.push(result_1.clone());
                std::mem::swap(&mut result_1, &mut result_2);
            }
            moves.push(base_move);

            if move_powers.len() == MAX_MOVE_POWER {
                return Err(KSolveConversionError::MoveOrderTooHigh);
            }

            let order = move_powers.len() as isize + 2;
            for (j, expanded_puzzle_state) in move_powers.into_iter().enumerate() {
                let mut twist: isize = j as isize + 2;
                if order - twist < twist {
                    twist -= order;
                }
                let mut expanded_name = base_name.clone();
                if twist != -1 {
                    expanded_name.push_str(&twist.abs().to_string());
                }
                if twist < 0 {
                    expanded_name.push('\'');
                }
                moves.push(Move {
                    puzzle_state: expanded_puzzle_state,
                    move_class_index,
                    name: expanded_name,
                });
            }
        }

        Ok(PuzzleDef {
            moves: moves.into_boxed_slice(),
            move_classes: move_classes.into_boxed_slice(),
            symmetries: symmetries.into_boxed_slice(),
            sorted_orbit_defs: sorted_orbit_defs.into_boxed_slice(),
            name: ksolve.name().to_owned(),
        })
    }
}

impl<const N: usize> PuzzleState for StackPuzzle<N> {
    type MultiBv = Box<[u8]>;

    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        new_multi_bv_slice(sorted_orbit_defs)
    }

    fn try_from_transformation_meta(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError> {
        if N < sorted_orbit_defs
            .iter()
            .map(|orbit_def| (orbit_def.piece_count.get() as usize) * 2)
            .sum()
        {
            return Err(KSolveConversionError::NotEnoughBufferSpace);
        }

        let mut orbit_states = [0_u8; N];
        ksolve_move_to_slice_unchecked(
            &mut orbit_states,
            sorted_orbit_defs,
            sorted_transformations,
        );
        Ok(StackPuzzle(orbit_states))
    }

    fn replace_compose(
        &mut self,
        a: &StackPuzzle<N>,
        b: &StackPuzzle<N>,
        sorted_orbit_defs: &[OrbitDef],
    ) {
        replace_compose_slice(&mut self.0, &a.0, &b.0, sorted_orbit_defs);
    }

    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: &[OrbitDef]) {
        replace_inverse_slice(&mut self.0, &a.0, sorted_orbit_defs);
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: &mut [u8],
    ) -> bool {
        induces_sorted_cycle_type_slice(&self.0, sorted_cycle_type, sorted_orbit_defs, multi_bv)
    }

    fn orbit_bytes_by_index(&self, index: usize, sorted_orbit_defs: &[OrbitDef]) -> (&[u8], &[u8]) {
        orbit_bytes_by_index_slice(&self.0, index, sorted_orbit_defs)
    }
}

impl PuzzleState for HeapPuzzle {
    type MultiBv = Box<[u8]>;

    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        new_multi_bv_slice(sorted_orbit_defs)
    }

    fn try_from_transformation_meta(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError> {
        // No validation needed. from_sorted_transformations_unchecked creates
        // an orbit states buffer that is guaranteed to be the right size, and
        // there is no restriction on the expected orbit defs
        let mut orbit_states = vec![
            0_u8;
            sorted_orbit_defs
                .iter()
                .map(|orbit_def| orbit_def.piece_count.get() as usize * 2)
                .sum()
        ]
        .into_boxed_slice();
        ksolve_move_to_slice_unchecked(
            &mut orbit_states,
            sorted_orbit_defs,
            sorted_transformations,
        );
        Ok(HeapPuzzle(orbit_states))
    }

    fn replace_compose(&mut self, a: &HeapPuzzle, b: &HeapPuzzle, sorted_orbit_defs: &[OrbitDef]) {
        replace_compose_slice(&mut self.0, &a.0, &b.0, sorted_orbit_defs);
    }

    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: &[OrbitDef]) {
        replace_inverse_slice(&mut self.0, &a.0, sorted_orbit_defs);
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: &mut [u8],
    ) -> bool {
        induces_sorted_cycle_type_slice(&self.0, sorted_cycle_type, sorted_orbit_defs, multi_bv)
    }

    fn orbit_bytes_by_index(&self, index: usize, sorted_orbit_defs: &[OrbitDef]) -> (&[u8], &[u8]) {
        orbit_bytes_by_index_slice(&self.0, index, sorted_orbit_defs)
    }
}

fn new_multi_bv_slice(sorted_orbit_defs: &[OrbitDef]) -> Box<[u8]> {
    vec![
        0;
        sorted_orbit_defs
            .last()
            .unwrap()
            .piece_count
            .get()
            .div_ceil(4) as usize
    ]
    .into_boxed_slice()
}

fn ksolve_move_to_slice_unchecked(
    orbit_states: &mut [u8],
    sorted_orbit_defs: &[OrbitDef],
    sorted_transformations: &[Vec<(u8, u8)>],
) {
    let mut i = 0;
    for (transformation, orbit_def) in sorted_transformations.iter().zip(sorted_orbit_defs.iter()) {
        let piece_count = orbit_def.piece_count.get() as usize;
        // TODO: make this more efficient:
        // - zero orientation mod optimization
        // - avoid the transformation for identities entirely
        if transformation.is_empty() {
            for j in 0..piece_count {
                orbit_states[i + j + piece_count] = 0;
                orbit_states[i + j] = j as u8;
            }
        } else {
            for j in 0..piece_count {
                let (perm, orientation_delta) = transformation[j];
                orbit_states[i + j + piece_count] = orientation_delta;
                orbit_states[i + j] = perm;
            }
        }
        i += piece_count * 2;
    }
}

fn replace_compose_slice(
    orbit_states_mut: &mut [u8],
    a: &[u8],
    b: &[u8],
    sorted_orbit_defs: &[OrbitDef],
) {
    debug_assert_eq!(
        sorted_orbit_defs
            .iter()
            .map(|orbit_def| (orbit_def.piece_count.get() as usize) * 2)
            .sum::<usize>(),
        orbit_states_mut.len()
    );
    debug_assert_eq!(orbit_states_mut.len(), a.len());
    debug_assert_eq!(a.len(), b.len());

    let mut base = 0;
    for &OrbitDef {
        piece_count,
        orientation_count,
    } in sorted_orbit_defs
    {
        let piece_count = piece_count.get() as usize;
        // SAFETY: Permutation vectors and orientation vectors are shuffled
        // around, based on code from twsearch [1]. Testing has shown this is
        // sound.
        // [1] https://github.com/cubing/twsearch
        if orientation_count == 1.try_into().unwrap() {
            for i in 0..piece_count {
                let base_i = base + i;
                unsafe {
                    let pos = *a.get_unchecked(base + *b.get_unchecked(base_i) as usize);
                    *orbit_states_mut.get_unchecked_mut(base_i) = pos;
                    *orbit_states_mut.get_unchecked_mut(base_i + piece_count) = 0;
                }
            }
        } else {
            for i in 0..piece_count {
                let base_i = base + i;
                unsafe {
                    let pos = a.get_unchecked(base + *b.get_unchecked(base_i) as usize);
                    let a_ori =
                        a.get_unchecked(base + *b.get_unchecked(base_i) as usize + piece_count);
                    let b_ori = b.get_unchecked(base_i + piece_count);
                    *orbit_states_mut.get_unchecked_mut(base_i) = *pos;
                    *orbit_states_mut.get_unchecked_mut(base_i + piece_count) =
                        (*a_ori + *b_ori) % orientation_count;
                }
            }
        }
        base += piece_count * 2;
    }
}

fn replace_inverse_slice(orbit_states_mut: &mut [u8], a: &[u8], sorted_orbit_defs: &[OrbitDef]) {
    debug_assert_eq!(
        sorted_orbit_defs
            .iter()
            .map(|orbit_def| (orbit_def.piece_count.get() as usize) * 2)
            .sum::<usize>(),
        orbit_states_mut.len()
    );
    debug_assert_eq!(orbit_states_mut.len(), a.len());

    let mut base = 0;
    for &OrbitDef {
        piece_count,
        orientation_count,
    } in sorted_orbit_defs
    {
        let piece_count = piece_count.get() as usize;
        // SAFETY: Permutation vectors and orientation vectors are shuffled
        // around, based on code from twsearch [1]. Testing has shown this is
        // sound.
        // [1] https://github.com/cubing/twsearch
        if orientation_count == 1.try_into().unwrap() {
            for i in 0..piece_count {
                let base_i = base + i;
                unsafe {
                    *orbit_states_mut.get_unchecked_mut(base + a[base_i] as usize) = i as u8;
                    *orbit_states_mut.get_unchecked_mut(base + a[base_i] as usize + piece_count) =
                        0;
                }
            }
        } else {
            for i in 0..piece_count {
                let base_i = base + i;
                unsafe {
                    *orbit_states_mut.get_unchecked_mut(base + a[base_i] as usize) = i as u8;
                    *orbit_states_mut.get_unchecked_mut(base + a[base_i] as usize + piece_count) =
                        (orientation_count.get() - a[base_i + piece_count]) % orientation_count;
                }
            }
        }
        base += piece_count * 2;
    }
}

fn induces_sorted_cycle_type_slice(
    orbit_states: &[u8],
    sorted_cycle_type: &[OrientedPartition],
    sorted_orbit_defs: &[OrbitDef],
    multi_bv: &mut [u8],
) -> bool {
    let mut base = 0;
    for (
        &OrbitDef {
            piece_count,
            orientation_count,
        },
        partition,
    ) in sorted_orbit_defs.iter().zip(sorted_cycle_type.iter())
    {
        multi_bv.fill(0);
        let mut covered_cycles_count = 0_u8;
        let piece_count = piece_count.get() as usize;
        for i in 0..piece_count {
            let (div, rem) = (i / 4, i % 4);
            // SAFETY: default_multi_bv_slice ensures that (i / 4) always fits
            // in multi_bv
            if unsafe { *multi_bv.get_unchecked(div) } & (1 << rem) != 0 {
                continue;
            }
            // SAFETY: see above
            unsafe {
                *multi_bv.get_unchecked_mut(div) |= 1 << rem;
            }
            let mut actual_cycle_length = 1;
            // SAFETY: sorted_orbit_defs guarantees that base (the orbit state
            // base pointer) + i (a permutation vector element) is valid
            let mut piece = unsafe { *orbit_states.get_unchecked(base + i) } as usize;
            // SAFETY: sorted_orbit_defs guarantees that base (the orbit state
            // base pointer) + i + piece (an orientation vector element) is valid
            let mut orientation_sum =
                unsafe { *orbit_states.get_unchecked(base + piece + piece_count) };

            while piece != i {
                actual_cycle_length += 1;
                let (div, rem) = (piece / 4, piece % 4);
                // SAFETY: default_multi_bv_slice ensures that (piece / 4)
                // always in fits in multi_bv
                unsafe {
                    *multi_bv.get_unchecked_mut(div) |= 1 << rem;
                }
                // SAFETY: sorted_orbit_defs guarantees that base (the orbit
                // state base pointer) + piece (a permutation vector element) is
                // valid
                unsafe {
                    piece = *orbit_states.get_unchecked(base + piece) as usize;
                }
                // SAFETY: sorted_orbit_defs guarantees that base (the orbit
                // state base pointer) + piece + piece_count (an orientation
                // vector element) is valid
                unsafe {
                    orientation_sum += *orbit_states.get_unchecked(base + piece + piece_count);
                }
            }

            let actual_orients = orientation_sum % orientation_count != 0;
            if actual_cycle_length == 1 && !actual_orients {
                continue;
            }
            let Some(valid_cycle_index) = partition.iter().enumerate().position(
                |(j, &(expected_cycle_length, expected_orients))| {
                    let (div, rem) = (j / 4, j % 4);
                    expected_cycle_length.get() == actual_cycle_length
                        && expected_orients == actual_orients
                        // SAFETY: default_multi_bv_slice ensures that (j / 4)
                        // always fits in multi_bv
                        && unsafe { *multi_bv.get_unchecked(div) } & (1 << (rem + 4)) == 0
                },
            ) else {
                return false;
            };
            let (div, rem) = (valid_cycle_index / 4, valid_cycle_index % 4);
            // SAFETY: default_multi_bv_slice ensures that
            // (valid_cycle_index / 4) always fits in multi_bv
            unsafe {
                *multi_bv.get_unchecked_mut(div) |= 1 << (rem + 4);
            }
            covered_cycles_count += 1;
            // cannot possibly return true if this runs
            if covered_cycles_count > partition.len() as u8 {
                return false;
            }
        }
        if covered_cycles_count != partition.len() as u8 {
            return false;
        }
        base += piece_count * 2;
    }
    true
}

fn orbit_bytes_by_index_slice<'a>(
    orbit_states: &'a [u8],
    index: usize,
    sorted_orbit_defs: &[OrbitDef],
) -> (&'a [u8], &'a [u8]) {
    todo!();
}

impl HeapPuzzle {
    /// Utility function for testing. Not optimized.
    pub fn cycle_type(
        &self,
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: &mut [u8],
    ) -> Vec<Vec<(NonZeroU8, bool)>> {
        let mut cycle_type = vec![];
        let mut base = 0;
        for &OrbitDef {
            piece_count,
            orientation_count,
        } in sorted_orbit_defs
        {
            let mut cycle_type_piece = vec![];
            multi_bv.fill(0);
            let piece_count = piece_count.get() as usize;
            for i in 0..piece_count {
                let (div, rem) = (i / 4, i % 4);
                if multi_bv[div] & (1 << rem) != 0 {
                    continue;
                }

                multi_bv[div] |= 1 << rem;
                let mut actual_cycle_length = 1;
                let mut piece = self.0[base + i] as usize;
                let mut orientation_sum = self.0[base + piece + piece_count];

                while piece != i {
                    actual_cycle_length += 1;
                    let (div, rem) = (piece / 4, piece % 4);
                    multi_bv[div] |= 1 << rem;
                    piece = self.0[base + piece] as usize;
                    orientation_sum += self.0[base + piece + piece_count];
                }

                let actual_orients = orientation_sum % orientation_count != 0;
                if actual_cycle_length != 1 || actual_orients {
                    cycle_type_piece
                        .push((NonZeroU8::new(actual_cycle_length).unwrap(), actual_orients))
                }
            }
            base += piece_count * 2;
            cycle_type_piece.sort();
            cycle_type.push(cycle_type_piece);
        }
        // We don't actually need to test this function because we have this
        assert!(self.induces_sorted_cycle_type(&cycle_type, sorted_orbit_defs, multi_bv));
        cycle_type
    }
}

pub fn random_3x3_state<P: PuzzleState>(cube3_def: &PuzzleDef<P>, solved: &P) -> P {
    let mut result_1 = solved.clone();
    let mut result_2 = solved.clone();
    for _ in 0..20 {
        let move_index = fastrand::choice(0_u8..18).unwrap();
        let move_ = &cube3_def.moves[move_index as usize];
        result_1.replace_compose(&result_2, &move_.puzzle_state, &cube3_def.sorted_orbit_defs);
        std::mem::swap(&mut result_2, &mut result_1);
    }
    result_2
}

#[cfg(test)]
mod tests {
    extern crate test;

    use super::*;
    use puzzle_geometry::ksolve::KPUZZLE_3X3;
    use test::Bencher;

    type StackCube3 = StackPuzzle<40>;

    fn ct(sorted_cycle_type: &[(u8, bool)]) -> OrientedPartition {
        sorted_cycle_type
            .iter()
            .map(|&(length, oriented)| (length.try_into().unwrap(), oriented))
            .collect()
    }

    pub fn apply_moves<P: PuzzleState + Clone>(
        puzzle_def: &PuzzleDef<P>,
        puzzle_state: &P,
        moves: &str,
        repeat: u32,
    ) -> P {
        let mut result_1 = puzzle_state.clone();
        let mut result_2 = puzzle_state.clone();

        for _ in 0..repeat {
            for name in moves.split_whitespace() {
                let move_ = puzzle_def.find_move(name).unwrap();
                result_2.replace_compose(
                    &result_1,
                    &move_.puzzle_state,
                    &puzzle_def.sorted_orbit_defs,
                );
                std::mem::swap(&mut result_1, &mut result_2);
            }
        }
        result_1
    }

    fn commutes_with<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut result_1 = cube3_def.new_solved_state();
        let mut result_2 = result_1.clone();

        let u_move = cube3_def.find_move("U").unwrap();
        let d2_move = cube3_def.find_move("D2").unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        assert!(u_move.commutes_with(
            u_move,
            &mut result_1,
            &mut result_2,
            &cube3_def.sorted_orbit_defs
        ));
        assert!(d2_move.commutes_with(
            d2_move,
            &mut result_1,
            &mut result_2,
            &cube3_def.sorted_orbit_defs
        ));
        assert!(u_move.commutes_with(
            d2_move,
            &mut result_1,
            &mut result_2,
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!u_move.commutes_with(
            r_move,
            &mut result_1,
            &mut result_2,
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!d2_move.commutes_with(
            r_move,
            &mut result_1,
            &mut result_2,
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!r_move.commutes_with(
            u_move,
            &mut result_1,
            &mut result_2,
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!r_move.commutes_with(
            d2_move,
            &mut result_1,
            &mut result_2,
            &cube3_def.sorted_orbit_defs
        ));
    }

    #[test]
    fn test_commutes_with() {
        commutes_with::<StackCube3>();
        commutes_with::<HeapPuzzle>();
        #[cfg(simd8and16)]
        commutes_with::<cube3::simd8and16::Cube3>();
        #[cfg(avx2)]
        commutes_with::<cube3::avx2::Cube3>();
    }

    #[test]
    fn test_not_enough_buffer_space() {
        let cube3_def = PuzzleDef::<StackPuzzle<39>>::try_from(&*KPUZZLE_3X3);
        assert!(matches!(
            cube3_def,
            Err(KSolveConversionError::NotEnoughBufferSpace)
        ));
    }

    pub fn many_compositions<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let also_solved = apply_moves(&cube3_def, &solved, "R F", 105);
        assert_eq!(also_solved, solved);
    }

    #[test]
    fn test_many_compositions() {
        many_compositions::<StackCube3>();
        many_compositions::<HeapPuzzle>();
        #[cfg(simd8and16)]
        many_compositions::<cube3::simd8and16::Cube3>();
        #[cfg(avx2)]
        many_compositions::<cube3::avx2::Cube3>();
    }

    pub fn s_u4_symmetry<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let s_u4_symmetry = cube3_def.find_symmetry("S_U4").unwrap();
        let solved = cube3_def.new_solved_state();

        let mut result_1 = solved.clone();
        let mut result_2 = solved.clone();
        for _ in 0..4 {
            result_2.replace_compose(
                &result_1,
                &s_u4_symmetry.puzzle_state,
                &cube3_def.sorted_orbit_defs,
            );
            std::mem::swap(&mut result_1, &mut result_2);
        }

        assert_eq!(result_1, solved);
    }

    #[test]
    fn test_s_u4_symmetry() {
        s_u4_symmetry::<StackCube3>();
        s_u4_symmetry::<HeapPuzzle>();
        #[cfg(simd8and16)]
        s_u4_symmetry::<cube3::simd8and16::Cube3>();
        #[cfg(avx2)]
        s_u4_symmetry::<cube3::avx2::Cube3>();
    }

    pub fn expanded_move<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let actual_solved = cube3_def.new_solved_state();
        let expected_solved = apply_moves(
            &cube3_def,
            &actual_solved,
            "R R' D2 D2 U U U2 F B' F' B",
            10,
        );
        assert_eq!(actual_solved, expected_solved);
    }

    #[test]
    fn test_expanded_move() {
        expanded_move::<StackCube3>();
        expanded_move::<HeapPuzzle>();
        #[cfg(simd8and16)]
        expanded_move::<cube3::simd8and16::Cube3>();
        #[cfg(avx2)]
        expanded_move::<cube3::avx2::Cube3>();
    }

    pub fn inversion<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let mut result = solved.clone();

        let state_r2_b_prime = apply_moves(&cube3_def, &solved, "R2 B'", 1);
        result.replace_inverse(&state_r2_b_prime, &cube3_def.sorted_orbit_defs);

        let state_b_r2 = apply_moves(&cube3_def, &solved, "B R2", 1);
        assert_eq!(result, state_b_r2);

        let in_r_f_cycle = apply_moves(&cube3_def, &solved, "R F", 40);
        result.replace_inverse(&in_r_f_cycle, &cube3_def.sorted_orbit_defs);

        let remaining_r_f_cycle = apply_moves(&cube3_def, &solved, "R F", 65);
        assert_eq!(result, remaining_r_f_cycle);

        for i in 1..=5 {
            let state = apply_moves(&cube3_def, &solved, "L F L' F'", i);
            result.replace_inverse(&state, &cube3_def.sorted_orbit_defs);
            let remaining_state = apply_moves(&cube3_def, &solved, "L F L' F'", 6 - i);
            assert_eq!(result, remaining_state);
        }
    }

    #[test]
    fn test_inversion() {
        inversion::<StackCube3>();
        inversion::<HeapPuzzle>();
        #[cfg(simd8and16)]
        inversion::<cube3::simd8and16::Cube3>();
        #[cfg(avx2)]
        inversion::<cube3::avx2::Cube3>();
    }

    pub fn random_inversion<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();

        for _ in 0..50 {
            let random_state = random_3x3_state(&cube3_def, &solved);
            let mut result_1 = solved.clone();
            let mut result_2 = solved.clone();
            result_1.replace_inverse(&random_state, &cube3_def.sorted_orbit_defs);
            result_2.replace_compose(&result_1, &random_state, &cube3_def.sorted_orbit_defs);

            assert_eq!(result_2, solved);
        }
    }

    #[test]
    fn test_random_inversion() {
        random_inversion::<StackCube3>();
        random_inversion::<HeapPuzzle>();
        #[cfg(simd8and16)]
        random_inversion::<cube3::simd8and16::Cube3>();
        #[cfg(avx2)]
        random_inversion::<cube3::avx2::Cube3>();
    }

    pub fn hash<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();

        let in_r_f_cycle_1 = apply_moves(&cube3_def, &solved, "R F", 40);
        let in_r_f_cycle_2 = apply_moves(&cube3_def, &solved, "F' R'", 65);

        assert_eq!(in_r_f_cycle_1, in_r_f_cycle_2);
        assert_eq!(fxhash::hash(&in_r_f_cycle_1), fxhash::hash(&in_r_f_cycle_2));
    }

    #[test]
    fn test_hash() {
        hash::<StackCube3>();
        hash::<HeapPuzzle>();
        #[cfg(simd8and16)]
        hash::<cube3::simd8and16::Cube3>();
        #[cfg(avx2)]
        hash::<cube3::avx2::Cube3>();
    }

    pub fn induces_sorted_cycle_type_within_cycle<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let mut multi_bv = P::new_multi_bv(&cube3_def.sorted_orbit_defs);

        let order_1260 = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 1);
        let sorted_cycle_type = [
            ct(&[(3, true), (5, true)]),
            ct(&[(2, false), (2, true), (7, true)]),
        ];
        assert!(order_1260.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let order_1260_in_cycle = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 209);
        assert!(order_1260_in_cycle.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
    }

    #[test]
    fn test_induces_sorted_cycle_type_within_cycle() {
        induces_sorted_cycle_type_within_cycle::<StackCube3>();
        induces_sorted_cycle_type_within_cycle::<HeapPuzzle>();
        #[cfg(simd8and16)]
        induces_sorted_cycle_type_within_cycle::<cube3::simd8and16::Cube3>();
        #[cfg(avx2)]
        induces_sorted_cycle_type_within_cycle::<cube3::avx2::Cube3>();
    }

    pub fn induces_sorted_cycle_type_many<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let mut multi_bv = P::new_multi_bv(&cube3_def.sorted_orbit_defs);
        assert!(solved.induces_sorted_cycle_type(
            &[vec![], vec![]],
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(
            &cube3_def,
            &solved,
            "F2 L' U2 F U F U L' B U' F' U D2 L F2 B'",
            1,
        );
        let sorted_cycle_type = [ct(&[(1, true), (3, true)]), ct(&[(1, true), (5, true)])];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(
            &cube3_def,
            &solved,
            "U2 L B L2 F U2 B' U2 R U' F R' F' R F' L' U2",
            1,
        );
        let sorted_cycle_type = [ct(&[(1, true), (5, true)]), ct(&[(1, true), (7, true)])];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(
            &cube3_def,
            &solved,
            "R' U2 R' U2 F' D' L F L2 F U2 F2 D' L' D2 F R2",
            1,
        );
        let sorted_cycle_type = [ct(&[(1, true), (3, true)]), ct(&[(1, true), (7, true)])];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(
            &cube3_def,
            &solved,
            "B2 U' B' D B' L' D' B U' R2 B2 R U B2 R B' R U",
            1,
        );
        let sorted_cycle_type = [
            ct(&[(1, true), (1, true), (3, true)]),
            ct(&[(1, true), (7, true)]),
        ];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(
            &cube3_def,
            &solved,
            "R2 L2 D' B L2 D' B L' B D2 R2 B2 R' D' B2 L2 U'",
            1,
        );
        let sorted_cycle_type = [ct(&[(2, true), (3, true)]), ct(&[(4, true), (5, true)])];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(
            &cube3_def,
            &solved,
            "F' B2 R L U2 B U2 L2 F2 U R L B' L' D' R' D' B'",
            1,
        );
        let sorted_cycle_type = [
            ct(&[(1, true), (2, true), (3, true)]),
            ct(&[(4, true), (5, true)]),
        ];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(
            &cube3_def,
            &solved,
            "L' D2 F B2 U F' L2 B R F2 D R' L F R' F' D",
            1,
        );
        let sorted_cycle_type = [
            ct(&[(2, true), (3, true)]),
            ct(&[(1, true), (4, true), (5, false)]),
        ];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(
            &cube3_def,
            &solved,
            "B' L' F2 R U' R2 F' L2 F R' L B L' U' F2 U' D2 L",
            1,
        );
        let sorted_cycle_type = [
            ct(&[(1, true), (2, true), (3, true)]),
            ct(&[(1, true), (4, true), (5, false)]),
        ];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(
            &cube3_def,
            &solved,
            "F2 D2 L' F D R2 F2 U2 L2 F R' B2 D2 R2 U R2 U",
            1,
        );
        let sorted_cycle_type = [
            ct(&[(1, true), (2, false), (3, true)]),
            ct(&[(4, true), (5, true)]),
        ];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(
            &cube3_def,
            &solved,
            "F2 B' R' F' L' D B' U' F U B' U2 D L' F' L' B R2",
            1,
        );
        let sorted_cycle_type = [
            ct(&[(1, true), (2, false), (3, true)]),
            ct(&[(1, true), (4, true), (5, false)]),
        ];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(&cube3_def, &solved, "U L U L2 U2 B2", 1);
        let sorted_cycle_type = [
            ct(&[(1, true), (2, false), (3, true)]),
            ct(&[(2, false), (3, false), (3, false)]),
        ];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));

        let random_state = apply_moves(&cube3_def, &solved, "U", 1);
        let sorted_cycle_type = [ct(&[(4, false)]), ct(&[(4, false)])];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            &cube3_def.sorted_orbit_defs,
            multi_bv.reusable_ref(),
        ));
    }

    #[test]
    fn test_induces_sorted_cycle_type_many() {
        induces_sorted_cycle_type_many::<StackCube3>();
        induces_sorted_cycle_type_many::<HeapPuzzle>();
        #[cfg(simd8and16)]
        induces_sorted_cycle_type_many::<cube3::simd8and16::Cube3>();
        #[cfg(avx2)]
        induces_sorted_cycle_type_many::<cube3::avx2::Cube3>();
    }

    pub fn bench_compose_helper<P: PuzzleState>(b: &mut Bencher) {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut solved = cube3_def.new_solved_state();
        let r_move = cube3_def.find_move("R").unwrap();
        let f_move = cube3_def.find_move("F").unwrap();
        b.iter(|| {
            test::black_box(&mut solved).replace_compose(
                test::black_box(&r_move.puzzle_state),
                test::black_box(&f_move.puzzle_state),
                &cube3_def.sorted_orbit_defs,
            );
        });
    }

    pub fn bench_inverse_helper<P: PuzzleState>(b: &mut Bencher) {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let mut result = solved.clone();
        let order_1260 = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 100);
        b.iter(|| {
            test::black_box(&mut result)
                .replace_inverse(test::black_box(&order_1260), &cube3_def.sorted_orbit_defs);
        });
    }

    pub fn bench_induces_sorted_cycle_type_worst_helper<P: PuzzleState>(b: &mut Bencher) {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let sorted_cycle_type = [
            ct(&[(3, true), (5, true)]),
            ct(&[(2, false), (2, true), (7, true)]),
        ];
        let order_1260 = apply_moves(&cube3_def, &cube3_def.new_solved_state(), "R U2 D' B D'", 1);
        let mut multi_bv = P::new_multi_bv(&cube3_def.sorted_orbit_defs);
        b.iter(|| {
            test::black_box(&order_1260).induces_sorted_cycle_type(
                test::black_box(&sorted_cycle_type),
                &cube3_def.sorted_orbit_defs,
                multi_bv.reusable_ref(),
            );
        });
    }

    pub fn bench_induces_sorted_cycle_type_average_helper<P: PuzzleState>(b: &mut Bencher) {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();

        let sorted_cycle_types = [
            [
                ct(&[(3, true), (5, true)]),
                ct(&[(2, false), (2, true), (7, true)]),
            ],
            [ct(&[(1, true), (3, true)]), ct(&[(1, true), (5, true)])],
            [ct(&[(2, true), (3, true)]), ct(&[(4, true), (5, true)])],
            [
                ct(&[(1, true), (2, true), (3, true)]),
                ct(&[(4, true), (5, true)]),
            ],
            [
                ct(&[(2, true), (3, true)]),
                ct(&[(1, true), (4, true), (5, false)]),
            ],
            [ct(&[(4, false)]), ct(&[(4, false)])],
        ];
        let sorted_cycle_types: Vec<_> =
            sorted_cycle_types.into_iter().cycle().take(1000).collect();
        let mut sorted_cycle_type_iter = sorted_cycle_types.iter().cycle();

        let random_1000: Vec<P> = (0..1000)
            .map(|_| random_3x3_state(&cube3_def, &solved))
            .collect();
        let mut random_iter = random_1000.iter().cycle();

        let mut multi_bv = P::new_multi_bv(&cube3_def.sorted_orbit_defs);
        b.iter(|| {
            test::black_box(unsafe { random_iter.next().unwrap_unchecked() })
                .induces_sorted_cycle_type(
                    test::black_box(unsafe { sorted_cycle_type_iter.next().unwrap_unchecked() }),
                    &cube3_def.sorted_orbit_defs,
                    multi_bv.reusable_ref(),
                );
        });
    }

    #[bench]
    fn bench_compose_cube3_heap(b: &mut Bencher) {
        bench_compose_helper::<HeapPuzzle>(b);
    }

    #[bench]
    fn bench_inverse_cube3_heap(b: &mut Bencher) {
        bench_inverse_helper::<HeapPuzzle>(b);
    }

    #[bench]
    fn bench_induces_sorted_cycle_type_cube3_heap_worst(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_worst_helper::<HeapPuzzle>(b);
    }

    #[bench]
    fn bench_induces_sorted_cycle_type_cube3_heap_average(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_average_helper::<HeapPuzzle>(b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_compose_cube3_simd8and16(b: &mut Bencher) {
        bench_compose_helper::<cube3::simd8and16::Cube3>(b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_inverse_cube3_simd8and16(b: &mut Bencher) {
        bench_inverse_helper::<cube3::simd8and16::Cube3>(b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_induces_sorted_cycle_type_cube3_simd8and16_worst(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_worst_helper::<cube3::simd8and16::Cube3>(b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_induces_sorted_cycle_type_cube3_simd8and16_average(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_average_helper::<cube3::simd8and16::Cube3>(b);
    }

    #[bench]
    #[cfg_attr(not(avx2), ignore)]
    fn bench_compose_cube3_avx2(b: &mut Bencher) {
        bench_compose_helper::<cube3::avx2::Cube3>(b);
    }

    #[bench]
    #[cfg_attr(not(avx2), ignore)]
    fn bench_inverse_cube3_avx2(b: &mut Bencher) {
        bench_inverse_helper::<cube3::avx2::Cube3>(b);
    }

    #[bench]
    #[cfg_attr(not(avx2), ignore)]
    fn bench_induces_sorted_cycle_type_cube3_avx2_worst(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_worst_helper::<cube3::avx2::Cube3>(b);
    }

    #[bench]
    #[cfg_attr(not(avx2), ignore)]
    fn bench_induces_sorted_cycle_type_cube3_avx2_average(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_average_helper::<cube3::avx2::Cube3>(b);
    }

    #[test]
    fn test_thing() {
        let cube3_def: PuzzleDef<HeapPuzzle> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let trans = apply_moves(&cube3_def, &solved, "U R U' F", 1);

        dbg!(trans);
    }
}
