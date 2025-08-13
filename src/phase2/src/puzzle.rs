use crate::{SliceView, SliceViewMut};
use generativity::{Guard, Id};
use itertools::Itertools;
use puzzle_geometry::ksolve::KSolve;
use std::{fmt::Debug, hash::Hash, num::NonZeroU8};
use thiserror::Error;

pub mod cube3;
pub mod slice_puzzle;

/// The puzzle state interface at the heart of the cycle combination solver.
/// Users may either use the generic `HeapPuzzle` implementor for any `KSolve`
/// definition or define fast puzzle-specific implementations, like Cube3.
pub trait PuzzleState<'id>: Clone + PartialEq + Debug {
    /// A reusable multi bit vector type to hold temporary storage in
    /// `induces_sorted_cycle_type`.
    type MultiBv: SliceViewMut;
    type OrbitBytesBuf<'a>: AsRef<[u8]>
    where
        Self: 'a + 'id;
    type OrbitIdentifier: OrbitIdentifier<'id> + Copy + Debug;

    /// Get a default multi bit vector for use in `induces_sorted_cycle_type`
    fn new_multi_bv(sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>) -> Self::MultiBv;

    /// Create a puzzle state from a sorted transformation and sorted
    /// orbit defs. `sorted_transformations` must to correspond to
    /// `sorted_orbit_defs`.
    ///
    /// # Errors
    ///
    /// If a puzzle state cannot be created from the orbit
    fn try_from_transformations_meta(
        transformations_meta: TransformationsMeta<'id, '_>,
        id: Id<'id>,
    ) -> Result<Self, TransformationsMetaError>;

    /// Compose two puzzle states in place.
    fn replace_compose(
        &mut self,
        a: &Self,
        b: &Self,
        sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
    );

    /// Inverse of a puzzle state.
    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>);

    /// The goal state for IDA* search.
    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: SortedCycleTypeRef<'id, '_>,
        sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
        multi_bv: <Self::MultiBv as SliceViewMut>::SliceMut<'_>,
    ) -> bool;

    /// Get the bytes of the specified orbit index in the form (permutation
    /// vector, orientation vector).
    fn orbit_bytes(
        &self,
        orbit_identifier: Self::OrbitIdentifier,
    ) -> (Self::OrbitBytesBuf<'_>, Self::OrbitBytesBuf<'_>);

    /// Return an integer that corresponds to a bijective mapping of the orbit
    /// identifier's states.
    fn exact_hasher_orbit(&self, orbit_identifier: Self::OrbitIdentifier) -> u64;

    /// Return a representation of the puzzle state that can be soundly hashed.
    fn approximate_hash_orbit(&self, orbit_identifier: Self::OrbitIdentifier) -> impl Hash;
}

// /// Get a usize that "identifies" an orbit. This is implementor-specific.
// /// For slice puzzles, the identifier is the starting index of the orbit data
// /// in the puzzle state buffer. For specific puzzles the identifier is the
// /// index of the orbit in the orbit definition.
// fn next_orbit_identifer(orbit_identifier: Self::OrbitIdentifier, orbit_def: BrandedOrbitDef) -> usize;

pub trait OrbitIdentifier<'id> {
    fn first_orbit_identifier(branded_orbit_def: BrandedOrbitDef<'id>) -> Self;

    #[must_use]
    fn next_orbit_identifier(self, branded_orbit_def: BrandedOrbitDef<'id>) -> Self;

    fn orbit_def(&self) -> OrbitDef;
}

// TODO: dont make everything public
#[derive(Debug)]
pub struct PuzzleDef<'id, P: PuzzleState<'id>> {
    pub moves: Box<[Move<'id, P>]>,
    // indicies into moves
    pub move_classes: Box<[usize]>,
    pub symmetries: Box<[Move<'id, P>]>,
    pub sorted_orbit_defs: Box<[OrbitDef]>,
    pub name: String,
    id: Id<'id>,
}

#[derive(Error, Debug)]
pub enum KSolveConversionError {
    #[error(
        "Phase 2 does not currently support puzzles with set sizes larger than 255, but it will in the future"
    )]
    SetSizeTooBig,
    #[error("Could not expand move set, order of a move too high")]
    MoveOrderTooHigh,
    #[error("Too many move classes")]
    TooManyMoveClasses,
    #[error("Invalid transformation while processing the KSolve definition: {0}")]
    TransformsMetaError(#[from] TransformationsMetaError),
}

#[derive(Debug, Clone)]
pub struct Move<'id, P: PuzzleState<'id>> {
    pub puzzle_state: P,
    pub move_class_index: usize,
    pub name: String,
    #[allow(dead_code)]
    id: Id<'id>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct OrbitDef {
    pub piece_count: NonZeroU8,
    pub orientation_count: NonZeroU8,
}

#[derive(Copy, Clone, Debug)]
pub struct BrandedOrbitDef<'id> {
    pub inner: OrbitDef,
    _id: Id<'id>,
}

#[derive(Clone, Copy)]
pub struct SortedOrbitDefsRef<'id, 'a> {
    pub inner: &'a [OrbitDef],
    id: Id<'id>,
}

#[derive(Clone)]
pub struct SortedCycleType<'id> {
    pub inner: Vec<Vec<(NonZeroU8, bool)>>,
    id: Id<'id>,
}

#[derive(Copy, Clone)]
pub struct SortedCycleTypeRef<'id, 'a> {
    pub inner: &'a [Vec<(NonZeroU8, bool)>],
    _id: Id<'id>,
}

#[derive(Clone, Copy)]
pub struct TransformationsMeta<'id, 'a> {
    sorted_transformations: &'a [Vec<(u8, u8)>],
    sorted_orbit_defs: SortedOrbitDefsRef<'id, 'a>,
}

#[derive(Error, Debug)]
pub enum SortedCycleTypeCreationError {
    #[error("Cycle type uses too many pieces, expected at most {expected} pieces but got {actual}")]
    TooManyPieces { expected: usize, actual: usize },
    #[error("Cycle type uses zero-length cycles, which is not allowed")]
    ZeroLengthCycle,
    #[error(
        "There must be the same number of cycle types as orbit definitions, expected {expected}, got {actual}"
    )]
    MismatchedLength { expected: usize, actual: usize },
}

#[derive(Error, Debug)]
pub enum TransformationsMetaError {
    #[error("Invalid KSolve orbit definitions. Expected: {expected:?}\nActual: {actual:?}")]
    InvalidOrbitDefs {
        expected: Vec<OrbitDef>,
        actual: Vec<OrbitDef>,
    },
    #[error("Not enough buffer space to convert move")]
    NotEnoughBufferSpace,
    #[error("Invalid set count, expected {expected} sets but got {actual}")]
    InvalidSetCount { expected: usize, actual: usize },
    #[error("Invalid piece count, expected {expected} pieces but got {actual}")]
    InvalidPieceCount { expected: u8, actual: usize },
    #[error(
        "Invalid orientation delta, expected a value between 0 and {expected} but got {actual}"
    )]
    InvalidOrientationDelta { expected: u8, actual: u8 },
    #[error("Permutation out of range, expected a value between 1 and {expected} but got {actual}")]
    PermutationOutOfRange { expected: u8, actual: u8 },
    #[error("Move is invalid: {0:?}")]
    InvalidTransformation(Vec<Vec<(u8, u8)>>),
}

impl<'id, 'a> TransformationsMeta<'id, 'a> {
    /// Create a `TransformationMeta` from `sorted_transformations` and
    /// `sorted_orbit_defs`.
    ///
    /// # Errors
    ///
    /// If the fields of the arguments are not valid. See
    /// `TransformationMetaError`.
    pub fn new(
        sorted_transformations: &'a [Vec<(u8, u8)>],
        sorted_orbit_defs: SortedOrbitDefsRef<'id, 'a>,
    ) -> Result<Self, TransformationsMetaError> {
        let actual_set_count = sorted_transformations.len();
        let expected_set_count = sorted_orbit_defs.inner.len();

        if sorted_transformations.len() != sorted_orbit_defs.inner.len() {
            return Err(TransformationsMetaError::InvalidSetCount {
                expected: expected_set_count,
                actual: actual_set_count,
            });
        }

        for (transformation, orbit_def) in sorted_transformations
            .iter()
            .zip(sorted_orbit_defs.branded_copied_iter())
        {
            let expected_piece_count = orbit_def.inner.piece_count.get();
            let actual_piece_count = transformation.len();

            if actual_piece_count != expected_piece_count as usize {
                return Err(TransformationsMetaError::InvalidPieceCount {
                    expected: expected_piece_count,
                    actual: actual_piece_count,
                });
            }

            let max_orientation_delta = orbit_def.inner.orientation_count.get() - 1;
            let mut covered_perms = vec![false; expected_piece_count as usize];

            for &(perm, orientation_delta) in transformation {
                if orientation_delta > max_orientation_delta {
                    return Err(TransformationsMetaError::InvalidOrientationDelta {
                        expected: max_orientation_delta,
                        actual: orientation_delta,
                    });
                }

                match covered_perms.get_mut(perm as usize) {
                    Some(i) => *i = true,
                    None => {
                        return Err(TransformationsMetaError::PermutationOutOfRange {
                            expected: expected_piece_count,
                            actual: perm,
                        });
                    }
                }
            }

            if covered_perms.iter().any(|&x| !x) {
                return Err(TransformationsMetaError::InvalidTransformation(
                    sorted_transformations.to_vec(),
                ));
            }
        }

        Ok(Self {
            sorted_transformations,
            sorted_orbit_defs,
        })
    }

    #[must_use]
    pub fn sorted_transformations(&self) -> &'a [Vec<(u8, u8)>] {
        self.sorted_transformations
    }

    #[must_use]
    pub fn sorted_orbit_defs(&self) -> SortedOrbitDefsRef<'id, 'a> {
        self.sorted_orbit_defs
    }
}

impl<'id> BrandedOrbitDef<'id> {
    #[must_use]
    pub fn new(orbit_def: OrbitDef, id: Id<'id>) -> Self {
        Self {
            inner: orbit_def,
            _id: id,
        }
    }
}

impl<'id> SortedCycleType<'id> {
    pub fn new(
        maybe_cycle_type: &[Vec<(u8, bool)>],
        sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
    ) -> Result<Self, SortedCycleTypeCreationError> {
        if maybe_cycle_type.len() != sorted_orbit_defs.inner.len() {
            return Err(SortedCycleTypeCreationError::MismatchedLength {
                expected: sorted_orbit_defs.inner.len(),
                actual: maybe_cycle_type.len(),
            });
        }

        let mut sorted_cycle_type = Vec::with_capacity(maybe_cycle_type.len());
        for (cycle_type, &orbit_def) in maybe_cycle_type.iter().zip(sorted_orbit_defs.inner) {
            let max_piece_count_sum = orbit_def.piece_count.get() as usize;
            let mut cycle_type_checked = Vec::with_capacity(cycle_type.len());
            let mut piece_count_sum = 0;
            for &(length, oriented) in cycle_type {
                if length == 1 && !oriented {
                    continue;
                }
                match NonZeroU8::new(length) {
                    Some(length) => {
                        piece_count_sum += length.get() as usize;
                        cycle_type_checked.push((length, oriented));
                    }
                    None => return Err(SortedCycleTypeCreationError::ZeroLengthCycle),
                }
            }
            if piece_count_sum > max_piece_count_sum {
                return Err(SortedCycleTypeCreationError::TooManyPieces {
                    expected: max_piece_count_sum,
                    actual: piece_count_sum,
                });
            }
            cycle_type_checked.sort_unstable();
            sorted_cycle_type.push(cycle_type_checked);
        }

        Ok(Self {
            inner: sorted_cycle_type,
            id: sorted_orbit_defs.id,
        })
    }
}

impl<'id> SortedOrbitDefsRef<'id, '_> {
    pub fn branded_copied_iter(&self) -> impl Iterator<Item = BrandedOrbitDef<'id>> {
        self.inner
            .iter()
            .copied()
            .map(|orbit_def| BrandedOrbitDef::new(orbit_def, self.id))
    }
}

// TODO
impl<'id> SliceView for SortedCycleType<'id> {
    type Slice<'a>
        = SortedCycleTypeRef<'id, 'a>
    where
        Self: 'a;

    fn slice_view(&self) -> Self::Slice<'_> {
        SortedCycleTypeRef {
            inner: &self.inner,
            _id: self.id,
        }
    }
}

impl<'id, P: PuzzleState<'id>> Move<'id, P> {
    /// # Safety
    ///
    /// `self` and `other` must both correspond to `sorted_orbit_defs`.
    pub fn commutes_with(
        &self,
        other: &Self,
        result_1: &mut P,
        result_2: &mut P,
        sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
    ) -> bool {
        result_1.replace_compose(&self.puzzle_state, &other.puzzle_state, sorted_orbit_defs);
        result_2.replace_compose(&other.puzzle_state, &self.puzzle_state, sorted_orbit_defs);
        result_1 == result_2
    }
}

fn solved_state_from_sorted_orbit_defs<'id, P: PuzzleState<'id>>(
    sorted_orbit_defs: &[OrbitDef],
    id: Id<'id>,
) -> P {
    let sorted_transformations = sorted_orbit_defs
        .iter()
        .copied()
        .map(|orbit_def| {
            (0..orbit_def.piece_count.get())
                .map(|i| (i, 0))
                .collect_vec()
        })
        .collect_vec();
    let sorted_orbit_defs_ref = SortedOrbitDefsRef {
        inner: sorted_orbit_defs,
        id,
    };
    let transformations_meta =
        TransformationsMeta::new(&sorted_transformations, sorted_orbit_defs_ref).unwrap();
    // We can unwrap because try_from guarantees that the orbit defs are valid
    P::try_from_transformations_meta(transformations_meta, id).unwrap()
}

impl<'id, P: PuzzleState<'id>> PuzzleDef<'id, P> {
    #[must_use]
    pub fn find_move(&self, name: &str) -> Option<&Move<'id, P>> {
        self.moves.iter().find(|move_| move_.name == name)
    }

    #[must_use]
    pub fn find_symmetry(&self, name: &str) -> Option<&Move<'id, P>> {
        self.symmetries.iter().find(|move_| move_.name == name)
    }

    #[must_use]
    pub fn new_solved_state(&self) -> P {
        solved_state_from_sorted_orbit_defs(&self.sorted_orbit_defs, self.id)
    }

    #[must_use]
    pub fn sorted_orbit_defs_slice_view(&self) -> SortedOrbitDefsRef<'id, '_> {
        SortedOrbitDefsRef {
            inner: &self.sorted_orbit_defs,
            id: self.id,
        }
    }

    // pub fn brand_orbit_def(&self, orbit_def: OrbitDef) -> BrandedOrbitDef<'id> {
    //     BrandedOrbitDef::new(orbit_def, self.id)
    // }

    /// Create a new `PuzzleDef` from a `KSolve` definition and a generativity
    /// `Guard`.
    ///
    /// # Errors
    ///
    /// The `KSolve` definition could not be converted to a `PuzzleDef`. See
    /// `KSolveConversionError`.
    pub fn new(
        ksolve: &KSolve,
        guard: Guard<'id>,
    ) -> Result<(Self, Id<'id>), KSolveConversionError> {
        let id = guard.into();
        let ksolve_orbit_defs: Vec<OrbitDef> = ksolve
            .sets()
            .iter()
            .map(|ksolve_set| {
                Ok(OrbitDef {
                    piece_count: ksolve_set
                        .piece_count()
                        .try_into()
                        .map_err(|_| KSolveConversionError::SetSizeTooBig)?,
                    // TODO: limit ori to 128 because of vectorized SlicePuzzle
                    // logic
                    orientation_count: ksolve_set.orientation_count(),
                })
            })
            .collect::<Result<_, KSolveConversionError>>()?;

        let mut arg_indicies = (0..ksolve_orbit_defs.len()).collect_vec();
        arg_indicies.sort_by_key(|&i| {
            (
                ksolve_orbit_defs[i].piece_count.get(),
                ksolve_orbit_defs[i].orientation_count.get(),
            )
        });

        let sorted_orbit_defs = arg_indicies
            .iter()
            .map(|&i| ksolve_orbit_defs[i])
            .collect_vec();

        let sorted_orbit_defs_ref = SortedOrbitDefsRef {
            inner: &sorted_orbit_defs,
            id,
        };

        let mut moves = Vec::with_capacity(ksolve.moves().len());
        let mut move_classes = vec![];
        let mut symmetries = Vec::with_capacity(ksolve.symmetries().len());

        for (i, ksolve_move) in ksolve
            .moves()
            .iter()
            .chain(ksolve.symmetries().iter())
            .enumerate()
        {
            const MAX_MOVE_POWER: usize = 1_000_000;

            let mut sorted_transformations = ksolve_move
                .transformation()
                .iter()
                .enumerate()
                .map(|(i, perm_and_ori)| {
                    if perm_and_ori.is_empty() {
                        (0..ksolve_orbit_defs[i].piece_count.get())
                            .map(|j| Ok((j, 0)))
                            .collect::<Result<Vec<_>, KSolveConversionError>>()
                    } else {
                        perm_and_ori
                            .iter()
                            .map(|&(perm, orientation)| {
                                // we can unwrap because sorted_orbit_defs exists
                                Ok((
                                    (perm.get() - 1)
                                        .try_into()
                                        .map_err(|_| KSolveConversionError::SetSizeTooBig)?,
                                    orientation,
                                ))
                            })
                            .collect::<Result<Vec<_>, KSolveConversionError>>()
                    }
                })
                .collect::<Result<Vec<Vec<_>>, KSolveConversionError>>()?;
            sorted_transformations = arg_indicies
                .iter()
                .map(|&i| sorted_transformations[i].clone())
                .collect();
            let transformations_meta =
                TransformationsMeta::new(&sorted_transformations, sorted_orbit_defs_ref)?;

            let puzzle_state = P::try_from_transformations_meta(transformations_meta, id)?;

            if i >= ksolve.moves().len() {
                let base_move = Move {
                    name: ksolve_move.name().to_owned(),
                    move_class_index: 0,
                    puzzle_state,
                    id,
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
                id,
            };

            let solved: P = solved_state_from_sorted_orbit_defs(&sorted_orbit_defs, id);

            let base_name = base_move.name.clone();
            move_classes.push(move_class);

            let mut move_powers: Vec<P> = vec![];
            for _ in 0..MAX_MOVE_POWER {
                result_1.replace_compose(&result_2, &base_move.puzzle_state, sorted_orbit_defs_ref);
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

            // MAX_MOVE_POWER is way less than isize::MAX
            #[allow(clippy::cast_possible_wrap)]
            let order = move_powers.len() as isize + 2;
            for (j, expanded_puzzle_state) in move_powers.into_iter().enumerate() {
                // see above
                #[allow(clippy::cast_possible_wrap)]
                let mut twist = j as isize + 2;
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
                    id,
                });
            }
        }

        Ok((
            PuzzleDef {
                moves: moves.into_boxed_slice(),
                move_classes: move_classes.into_boxed_slice(),
                symmetries: symmetries.into_boxed_slice(),
                sorted_orbit_defs: sorted_orbit_defs.into_boxed_slice(),
                name: ksolve.name().to_owned(),
                id,
            },
            id,
        ))
    }
}

impl SliceViewMut for () {
    type SliceMut<'a> = ();

    fn slice_view_mut(&mut self) -> Self::SliceMut<'_> {}
}

/// A utility function for testing. Not optimized.
///
/// # Panics
///
/// Panics if the move sequence is invalid.
pub fn apply_moves<'id, P: PuzzleState<'id>>(
    puzzle_def: &PuzzleDef<'id, P>,
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
                puzzle_def.sorted_orbit_defs_slice_view(),
            );
            std::mem::swap(&mut result_1, &mut result_2);
        }
    }
    result_1
}

/// Return a random 3x3 puzzle state
#[allow(clippy::missing_panics_doc)]
pub fn apply_random_moves<'id, P: PuzzleState<'id>>(
    puzzle_def: &PuzzleDef<'id, P>,
    solved: &P,
    random_move_count: u32,
) -> P {
    let mut result_1 = solved.clone();
    let mut result_2 = solved.clone();
    for _ in 0..random_move_count {
        let move_ = fastrand::choice(puzzle_def.moves.iter()).unwrap();
        result_1.replace_compose(
            &result_2,
            &move_.puzzle_state,
            puzzle_def.sorted_orbit_defs_slice_view(),
        );
        std::mem::swap(&mut result_2, &mut result_1);
    }
    result_2
}

#[cfg(test)]
mod tests {
    extern crate test;

    use super::{
        slice_puzzle::{HeapPuzzle, StackPuzzle},
        *,
    };
    use generativity::make_guard;
    use puzzle_geometry::ksolve::KPUZZLE_3X3;
    use test::Bencher;

    type StackCube3<'id> = StackPuzzle<'id, 40>;

    fn commutes_with<'id, P: PuzzleState<'id>>(guard: Guard<'id>) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let mut result_1 = cube3_def.new_solved_state();
        let mut result_2 = result_1.clone();

        let u_move = cube3_def.find_move("U").unwrap();
        let d2_move = cube3_def.find_move("D2").unwrap();
        let r_move = cube3_def.find_move("R").unwrap();

        assert!(u_move.commutes_with(
            u_move,
            &mut result_1,
            &mut result_2,
            cube3_def.sorted_orbit_defs_slice_view()
        ));
        assert!(d2_move.commutes_with(
            d2_move,
            &mut result_1,
            &mut result_2,
            cube3_def.sorted_orbit_defs_slice_view()
        ));
        assert!(u_move.commutes_with(
            d2_move,
            &mut result_1,
            &mut result_2,
            cube3_def.sorted_orbit_defs_slice_view()
        ));
        assert!(!u_move.commutes_with(
            r_move,
            &mut result_1,
            &mut result_2,
            cube3_def.sorted_orbit_defs_slice_view()
        ));
        assert!(!d2_move.commutes_with(
            r_move,
            &mut result_1,
            &mut result_2,
            cube3_def.sorted_orbit_defs_slice_view()
        ));
        assert!(!r_move.commutes_with(
            u_move,
            &mut result_1,
            &mut result_2,
            cube3_def.sorted_orbit_defs_slice_view()
        ));
        assert!(!r_move.commutes_with(
            d2_move,
            &mut result_1,
            &mut result_2,
            cube3_def.sorted_orbit_defs_slice_view()
        ));
    }

    #[test]
    fn test_commutes_with() {
        make_guard!(guard);
        commutes_with::<StackCube3>(guard);
        make_guard!(guard);
        commutes_with::<HeapPuzzle>(guard);
        #[cfg(simd8and16)]
        {
            make_guard!(guard);
            commutes_with::<cube3::simd8and16::Cube3>(guard);
            make_guard!(guard);
            commutes_with::<cube3::simd8and16::UncompressedCube3>(guard);
        }
        #[cfg(avx2)]
        {
            make_guard!(guard);
            commutes_with::<cube3::avx2::Cube3>(guard);
        }
    }

    #[test]
    fn test_not_enough_buffer_space() {
        make_guard!(guard);
        let try_cube3_def = PuzzleDef::<StackPuzzle<39>>::new(&KPUZZLE_3X3, guard);
        assert!(matches!(
            try_cube3_def,
            Err(KSolveConversionError::TransformsMetaError(
                TransformationsMetaError::NotEnoughBufferSpace
            ))
        ));
    }

    pub fn many_compositions<'id, P: PuzzleState<'id>>(guard: Guard<'id>) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let solved = cube3_def.new_solved_state();
        let also_solved = apply_moves(&cube3_def, &solved, "R F", 105);
        assert_eq!(also_solved, solved);
    }

    #[test]
    fn test_many_compositions() {
        make_guard!(guard);
        many_compositions::<StackCube3>(guard);
        make_guard!(guard);
        many_compositions::<HeapPuzzle>(guard);
        #[cfg(simd8and16)]
        {
            make_guard!(guard);
            many_compositions::<cube3::simd8and16::Cube3>(guard);
            make_guard!(guard);
            many_compositions::<cube3::simd8and16::UncompressedCube3>(guard);
        }
        #[cfg(avx2)]
        {
            make_guard!(guard);
            many_compositions::<cube3::avx2::Cube3>(guard);
        }
    }

    pub fn s_u4_symmetry<'id, P: PuzzleState<'id>>(guard: Guard<'id>) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let s_u4_symmetry = cube3_def.find_symmetry("S_U4").unwrap();
        let solved = cube3_def.new_solved_state();

        let mut result_1 = solved.clone();
        let mut result_2 = solved.clone();
        for _ in 0..4 {
            result_2.replace_compose(
                &result_1,
                &s_u4_symmetry.puzzle_state,
                cube3_def.sorted_orbit_defs_slice_view(),
            );
            std::mem::swap(&mut result_1, &mut result_2);
        }

        assert_eq!(result_1, solved);
    }

    #[test]
    fn test_s_u4_symmetry() {
        make_guard!(guard);
        s_u4_symmetry::<StackCube3>(guard);
        make_guard!(guard);
        s_u4_symmetry::<HeapPuzzle>(guard);
        #[cfg(simd8and16)]
        {
            make_guard!(guard);
            s_u4_symmetry::<cube3::simd8and16::Cube3>(guard);
            make_guard!(guard);
            s_u4_symmetry::<cube3::simd8and16::UncompressedCube3>(guard);
        }
        #[cfg(avx2)]
        {
            make_guard!(guard);
            s_u4_symmetry::<cube3::avx2::Cube3>(guard);
        }
    }

    pub fn expanded_move<'id, P: PuzzleState<'id>>(guard: Guard<'id>) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
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
        make_guard!(guard);
        expanded_move::<StackCube3>(guard);
        make_guard!(guard);
        expanded_move::<HeapPuzzle>(guard);
        #[cfg(simd8and16)]
        {
            make_guard!(guard);
            expanded_move::<cube3::simd8and16::Cube3>(guard);
            make_guard!(guard);
            expanded_move::<cube3::simd8and16::UncompressedCube3>(guard);
        }
        #[cfg(avx2)]
        {
            make_guard!(guard);
            expanded_move::<cube3::avx2::Cube3>(guard);
        }
    }

    pub fn inversion<'id, P: PuzzleState<'id>>(guard: Guard<'id>) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let solved = cube3_def.new_solved_state();
        let mut result = solved.clone();

        let state_r2_b_prime = apply_moves(&cube3_def, &solved, "R2 B'", 1);
        result.replace_inverse(&state_r2_b_prime, cube3_def.sorted_orbit_defs_slice_view());

        let state_b_r2 = apply_moves(&cube3_def, &solved, "B R2", 1);
        assert_eq!(result, state_b_r2);

        let in_r_f_cycle = apply_moves(&cube3_def, &solved, "R F", 40);
        result.replace_inverse(&in_r_f_cycle, cube3_def.sorted_orbit_defs_slice_view());

        let remaining_r_f_cycle = apply_moves(&cube3_def, &solved, "R F", 65);
        assert_eq!(result, remaining_r_f_cycle);

        for i in 1..=5 {
            let state = apply_moves(&cube3_def, &solved, "L F L' F'", i);
            result.replace_inverse(&state, cube3_def.sorted_orbit_defs_slice_view());
            let remaining_state = apply_moves(&cube3_def, &solved, "L F L' F'", 6 - i);
            assert_eq!(result, remaining_state);
        }
    }

    #[test]
    fn test_inversion() {
        make_guard!(guard);
        inversion::<StackCube3>(guard);
        make_guard!(guard);
        inversion::<HeapPuzzle>(guard);
        #[cfg(simd8and16)]
        {
            make_guard!(guard);
            inversion::<cube3::simd8and16::Cube3>(guard);
            make_guard!(guard);
            inversion::<cube3::simd8and16::UncompressedCube3>(guard);
        }
        #[cfg(avx2)]
        {
            make_guard!(guard);
            inversion::<cube3::avx2::Cube3>(guard);
        }
    }

    pub fn random_inversion<'id, P: PuzzleState<'id>>(guard: Guard<'id>) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let solved = cube3_def.new_solved_state();

        for _ in 0..50 {
            let random_state = apply_random_moves(&cube3_def, &solved, 20);
            let mut result_1 = solved.clone();
            let mut result_2 = solved.clone();
            result_1.replace_inverse(&random_state, cube3_def.sorted_orbit_defs_slice_view());
            result_2.replace_compose(
                &result_1,
                &random_state,
                cube3_def.sorted_orbit_defs_slice_view(),
            );

            assert_eq!(result_2, solved);
        }
    }

    #[test]
    fn test_random_inversion() {
        make_guard!(guard);
        random_inversion::<StackCube3>(guard);
        make_guard!(guard);
        random_inversion::<HeapPuzzle>(guard);
        #[cfg(simd8and16)]
        {
            make_guard!(guard);
            random_inversion::<cube3::simd8and16::Cube3>(guard);
            make_guard!(guard);
            random_inversion::<cube3::simd8and16::UncompressedCube3>(guard);
        }
        #[cfg(avx2)]
        {
            make_guard!(guard);
            random_inversion::<cube3::avx2::Cube3>(guard);
        }
    }

    pub fn induces_sorted_cycle_type_within_cycle<'id, P: PuzzleState<'id>>(guard: Guard<'id>) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let solved = cube3_def.new_solved_state();
        let mut multi_bv = P::new_multi_bv(cube3_def.sorted_orbit_defs_slice_view());

        let order_1260 = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 1);
        let sorted_cycle_type = SortedCycleType::new(
            &[
                vec![(3, true), (5, true)],
                vec![(2, false), (2, true), (7, true)],
            ],
            cube3_def.sorted_orbit_defs_slice_view(),
        )
        .unwrap();
        assert!(order_1260.induces_sorted_cycle_type(
            sorted_cycle_type.slice_view(),
            cube3_def.sorted_orbit_defs_slice_view(),
            multi_bv.slice_view_mut(),
        ));

        let order_1260_in_cycle = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 209);
        assert!(order_1260_in_cycle.induces_sorted_cycle_type(
            sorted_cycle_type.slice_view(),
            cube3_def.sorted_orbit_defs_slice_view(),
            multi_bv.slice_view_mut(),
        ));
    }

    #[test]
    fn test_induces_sorted_cycle_type_within_cycle() {
        make_guard!(guard);
        induces_sorted_cycle_type_within_cycle::<StackCube3>(guard);
        make_guard!(guard);
        induces_sorted_cycle_type_within_cycle::<HeapPuzzle>(guard);
        #[cfg(simd8and16)]
        {
            make_guard!(guard);
            induces_sorted_cycle_type_within_cycle::<cube3::simd8and16::Cube3>(guard);
            make_guard!(guard);
            induces_sorted_cycle_type_within_cycle::<cube3::simd8and16::UncompressedCube3>(guard);
        }
        #[cfg(avx2)]
        {
            make_guard!(guard);
            induces_sorted_cycle_type_within_cycle::<cube3::avx2::Cube3>(guard);
        }
    }

    pub fn induces_sorted_cycle_type_many<'id, P: PuzzleState<'id>>(guard: Guard<'id>) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let solved = cube3_def.new_solved_state();
        let mut multi_bv = P::new_multi_bv(cube3_def.sorted_orbit_defs_slice_view());

        let sorted_cycle_type = SortedCycleType::new(
            &[vec![], vec![]],
            cube3_def.sorted_orbit_defs_slice_view(),
        )
        .unwrap();
        assert!(solved.induces_sorted_cycle_type(
            sorted_cycle_type.slice_view(),
            cube3_def.sorted_orbit_defs_slice_view(),
            multi_bv.slice_view_mut(),
        ));

        let tests = [
            (
                "F2 L' U2 F U F U L' B U' F' U D2 L F2 B'",
                SortedCycleType::new(
                    &[vec![(1, true), (3, true)], vec![(1, true), (5, true)]],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
            (
                "U2 L B L2 F U2 B' U2 R U' F R' F' R F' L' U2",
                SortedCycleType::new(
                    &[vec![(1, true), (5, true)], vec![(1, true), (7, true)]],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
            (
                "R' U2 R' U2 F' D' L F L2 F U2 F2 D' L' D2 F R2",
                SortedCycleType::new(
                    &[vec![(1, true), (3, true)], vec![(1, true), (7, true)]],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
            (
                "B2 U' B' D B' L' D' B U' R2 B2 R U B2 R B' R U",
                SortedCycleType::new(
                    &[
                        vec![(1, true), (1, true), (3, true)],
                        vec![(1, true), (7, true)],
                    ],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
            (
                "R2 L2 D' B L2 D' B L' B D2 R2 B2 R' D' B2 L2 U'",
                SortedCycleType::new(
                    &[vec![(2, true), (3, true)], vec![(4, true), (5, true)]],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
            (
                "F' B2 R L U2 B U2 L2 F2 U R L B' L' D' R' D' B'",
                SortedCycleType::new(
                    &[
                        vec![(1, true), (2, true), (3, true)],
                        vec![(4, true), (5, true)],
                    ],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
            (
                "L' D2 F B2 U F' L2 B R F2 D R' L F R' F' D",
                SortedCycleType::new(
                    &[
                        vec![(2, true), (3, true)],
                        vec![(1, true), (4, true), (5, false)],
                    ],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
            (
                "B' L' F2 R U' R2 F' L2 F R' L B L' U' F2 U' D2 L",
                SortedCycleType::new(
                    &[
                        vec![(1, true), (2, true), (3, true)],
                        vec![(1, true), (4, true), (5, false)],
                    ],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
            (
                "F2 D2 L' F D R2 F2 U2 L2 F R' B2 D2 R2 U R2 U",
                SortedCycleType::new(
                    &[
                        vec![(1, true), (2, false), (3, true)],
                        vec![(4, true), (5, true)],
                    ],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
            (
                "F2 B' R' F' L' D B' U' F U B' U2 D L' F' L' B R2",
                SortedCycleType::new(
                    &[
                        vec![(1, true), (2, false), (3, true)],
                        vec![(1, true), (4, true), (5, false)],
                    ],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
            (
                "U L U L2 U2 B2",
                SortedCycleType::new(
                    &[
                        vec![(1, true), (2, false), (3, true)],
                        vec![(2, false), (3, false), (3, false)],
                    ],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
            (
                "U",
                SortedCycleType::new(
                    &[vec![(4, false)], vec![(4, false)]],
                    cube3_def.sorted_orbit_defs_slice_view(),
                )
                .unwrap(),
            ),
        ];

        for (i, (moves_str, expected_cts)) in tests.iter().enumerate() {
            let random_state = apply_moves(&cube3_def, &solved, moves_str, 1);

            assert!(random_state.induces_sorted_cycle_type(
                expected_cts.slice_view(),
                cube3_def.sorted_orbit_defs_slice_view(),
                multi_bv.slice_view_mut(),
            ));

            assert!(!solved.induces_sorted_cycle_type(
                expected_cts.slice_view(),
                cube3_def.sorted_orbit_defs_slice_view(),
                multi_bv.slice_view_mut(),
            ));

            for (j, &(other_moves, _)) in tests.iter().enumerate() {
                if i == j {
                    continue;
                }
                let other_state = apply_moves(&cube3_def, &solved, other_moves, 1);
                assert!(!other_state.induces_sorted_cycle_type(
                    expected_cts.slice_view(),
                    cube3_def.sorted_orbit_defs_slice_view(),
                    multi_bv.slice_view_mut()
                ));
            }
        }
    }

    #[test]
    fn test_induces_sorted_cycle_type_many() {
        make_guard!(guard);
        induces_sorted_cycle_type_many::<StackCube3>(guard);
        make_guard!(guard);
        induces_sorted_cycle_type_many::<HeapPuzzle>(guard);
        #[cfg(simd8and16)]
        {
            make_guard!(guard);
            induces_sorted_cycle_type_many::<cube3::simd8and16::Cube3>(guard);
            make_guard!(guard);
            induces_sorted_cycle_type_many::<cube3::simd8and16::UncompressedCube3>(guard);
        }
        #[cfg(avx2)]
        {
            make_guard!(guard);
            induces_sorted_cycle_type_many::<cube3::avx2::Cube3>(guard);
        }
    }

    fn exact_hasher_orbit<'id, P: PuzzleState<'id>>(guard: Guard<'id>) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let solved = cube3_def.new_solved_state();

        for (test_state, exp_hashes) in [
            (solved.clone(), [0, 0]),
            (
                apply_moves(&cube3_def, &solved, "U", 1),
                [24_476_904, 161_792],
            ),
            (
                apply_moves(&cube3_def, &solved, "U", 2),
                [57_868_020, 219_136],
            ),
            (
                apply_moves(&cube3_def, &solved, "R U R' U'", 1),
                [11_876_463, 825_765_658_624],
            ),
            (
                apply_moves(&cube3_def, &solved, "R U2 D' B D'", 1),
                [61_275_986, 279_798_716_817],
            ),
            (
                apply_moves(
                    &cube3_def,
                    &solved,
                    "B2 U' B' D B' L' D' B U' R2 B2 R U B2 R B' R U",
                    1,
                ),
                [857_489, 7_312_476_362],
            ),
            (
                apply_moves(
                    &cube3_def,
                    &solved,
                    "F2 B' R' F' L' D B' U' F U B' U2 D L' F' L' B R2",
                    1,
                ),
                [79_925_404, 38_328_854_695],
            ),
        ] {
            let mut maybe_orbit_identifier: Option<P::OrbitIdentifier> = None;
            for (i, branded_orbit_def) in cube3_def
                .sorted_orbit_defs_slice_view()
                .branded_copied_iter()
                .enumerate()
            {
                maybe_orbit_identifier = Some(if i == 0 {
                    P::OrbitIdentifier::first_orbit_identifier(branded_orbit_def)
                } else {
                    maybe_orbit_identifier
                        .unwrap()
                        .next_orbit_identifier(branded_orbit_def)
                });
                let orbit_identifier = maybe_orbit_identifier.unwrap();
                let hash = test_state.exact_hasher_orbit(orbit_identifier);
                assert_eq!(hash, exp_hashes[i]);
            }
        }
    }

    #[test]
    fn test_exact_hasher_orbit() {
        make_guard!(guard);
        exact_hasher_orbit::<StackCube3>(guard);
        make_guard!(guard);
        exact_hasher_orbit::<HeapPuzzle>(guard);
        #[cfg(simd8and16)]
        {
            make_guard!(guard);
            many_compositions::<cube3::simd8and16::Cube3>(guard);
            make_guard!(guard);
            exact_hasher_orbit::<cube3::simd8and16::UncompressedCube3>(guard);
        }
        #[cfg(avx2)]
        {
            make_guard!(guard);
            exact_hasher_orbit::<cube3::avx2::Cube3>(guard);
        }
    }

    pub fn bench_compose_helper<'id, P: PuzzleState<'id>>(guard: Guard<'id>, b: &mut Bencher) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let mut solved = cube3_def.new_solved_state();
        let r_move = cube3_def.find_move("R").unwrap();
        let f_move = cube3_def.find_move("F").unwrap();
        b.iter(|| {
            test::black_box(&mut solved).replace_compose(
                test::black_box(&r_move.puzzle_state),
                test::black_box(&f_move.puzzle_state),
                cube3_def.sorted_orbit_defs_slice_view(),
            );
        });
    }

    pub fn bench_inverse_helper<'id, P: PuzzleState<'id>>(guard: Guard<'id>, b: &mut Bencher) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let solved = cube3_def.new_solved_state();
        let mut result = solved.clone();
        let order_1260 = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 100);
        b.iter(|| {
            test::black_box(&mut result).replace_inverse(
                test::black_box(&order_1260),
                cube3_def.sorted_orbit_defs_slice_view(),
            );
        });
    }

    pub fn bench_induces_sorted_cycle_type_worst_helper<'id, P: PuzzleState<'id>>(
        guard: Guard<'id>,
        b: &mut Bencher,
    ) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let sorted_cycle_type = SortedCycleType::new(
            &[
                vec![(3, true), (5, true)],
                vec![(2, false), (2, true), (7, true)],
            ],
            cube3_def.sorted_orbit_defs_slice_view(),
        )
        .unwrap();
        let order_1260 = apply_moves(&cube3_def, &cube3_def.new_solved_state(), "R U2 D' B D'", 1);
        let mut multi_bv = P::new_multi_bv(cube3_def.sorted_orbit_defs_slice_view());
        b.iter(|| {
            test::black_box(&order_1260).induces_sorted_cycle_type(
                test::black_box(sorted_cycle_type.slice_view()),
                cube3_def.sorted_orbit_defs_slice_view(),
                multi_bv.slice_view_mut(),
            );
        });
    }

    pub fn bench_induces_sorted_cycle_type_average_helper<'id, P: PuzzleState<'id>>(
        guard: Guard<'id>,
        b: &mut Bencher,
    ) {
        let cube3_def = PuzzleDef::<P>::new(&KPUZZLE_3X3, guard).unwrap().0;
        let solved = cube3_def.new_solved_state();

        // let sorted_cycle_types = [
        //     [
        //         ct(&[(3, true), (5, true)]),
        //         ct(&[(2, false), (2, true), (7, true)]),
        //     ],
        //     [ct(&[(1, true), (3, true)]), ct(&[(1, true), (5, true)])],
        //     [ct(&[(2, true), (3, true)]), ct(&[(4, true), (5, true)])],
        //     [
        //         ct(&[(1, true), (2, true), (3, true)]),
        //         ct(&[(4, true), (5, true)]),
        //     ],
        //     [
        //         ct(&[(2, true), (3, true)]),
        //         ct(&[(1, true), (4, true), (5, false)]),
        //     ],
        //     [ct(&[(4, false)]), ct(&[(4, false)])],
        // ];
        let sorted_cycle_types = [
            SortedCycleType::new(
                &[
                    vec![(3, true), (5, true)],
                    vec![(2, false), (2, true), (7, true)],
                ],
                cube3_def.sorted_orbit_defs_slice_view(),
            )
            .unwrap(),
            SortedCycleType::new(
                &[vec![(1, true), (3, true)], vec![(1, true), (5, true)]],
                cube3_def.sorted_orbit_defs_slice_view(),
            )
            .unwrap(),
            SortedCycleType::new(
                &[vec![(2, true), (3, true)], vec![(4, true), (5, true)]],
                cube3_def.sorted_orbit_defs_slice_view(),
            )
            .unwrap(),
            SortedCycleType::new(
                &[
                    vec![(1, true), (2, true), (3, true)],
                    vec![(4, true), (5, true)],
                ],
                cube3_def.sorted_orbit_defs_slice_view(),
            )
            .unwrap(),
            SortedCycleType::new(
                &[
                    vec![(2, true), (3, true)],
                    vec![(1, true), (4, true), (5, false)],
                ],
                cube3_def.sorted_orbit_defs_slice_view(),
            )
            .unwrap(),
            SortedCycleType::new(
                &[vec![(4, false)], vec![(4, false)]],
                cube3_def.sorted_orbit_defs_slice_view(),
            )
            .unwrap(),
        ];
        let sorted_cycle_types: Vec<_> =
            sorted_cycle_types.into_iter().cycle().take(1000).collect();
        let mut sorted_cycle_type_iter = sorted_cycle_types.iter().cycle();

        let random_1000: Vec<P> = (0..1000)
            .map(|_| apply_random_moves(&cube3_def, &solved, 20))
            .collect();
        let mut random_iter = random_1000.iter().cycle();

        let mut multi_bv = P::new_multi_bv(cube3_def.sorted_orbit_defs_slice_view());
        b.iter(|| {
            test::black_box(unsafe { random_iter.next().unwrap_unchecked() })
                .induces_sorted_cycle_type(
                    test::black_box(unsafe {
                        sorted_cycle_type_iter
                            .next()
                            .unwrap_unchecked()
                            .slice_view()
                    }),
                    cube3_def.sorted_orbit_defs_slice_view(),
                    multi_bv.slice_view_mut(),
                );
        });
    }

    // --- HeapPuzzle benchmarks ---

    #[bench]
    fn bench_compose_cube3_heap(b: &mut Bencher) {
        make_guard!(guard);
        bench_compose_helper::<HeapPuzzle>(guard, b);
    }

    #[bench]
    fn bench_inverse_cube3_heap(b: &mut Bencher) {
        make_guard!(guard);
        bench_inverse_helper::<HeapPuzzle>(guard, b);
    }

    #[bench]
    fn bench_induces_sorted_cycle_type_cube3_heap_worst(b: &mut Bencher) {
        make_guard!(guard);
        bench_induces_sorted_cycle_type_worst_helper::<HeapPuzzle>(guard, b);
    }

    #[bench]
    fn bench_induces_sorted_cycle_type_cube3_heap_average(b: &mut Bencher) {
        make_guard!(guard);
        bench_induces_sorted_cycle_type_average_helper::<HeapPuzzle>(guard, b);
    }

    // --- simd8and16::UncompressedCube3 benchmarks ---

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_compose_uncompressed_cube3_simd8and16(b: &mut Bencher) {
        make_guard!(guard);
        bench_compose_helper::<cube3::simd8and16::UncompressedCube3>(guard, b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_inverse_uncompressed_cube3_simd8and16(b: &mut Bencher) {
        make_guard!(guard);
        bench_inverse_helper::<cube3::simd8and16::UncompressedCube3>(guard, b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_induces_sorted_cycle_type_uncompressed_cube3_simd8and16_worst(b: &mut Bencher) {
        make_guard!(guard);
        bench_induces_sorted_cycle_type_worst_helper::<cube3::simd8and16::UncompressedCube3>(
            guard, b,
        );
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_induces_sorted_cycle_type_uncompressed_cube3_simd8and16_average(b: &mut Bencher) {
        make_guard!(guard);
        bench_induces_sorted_cycle_type_average_helper::<cube3::simd8and16::UncompressedCube3>(
            guard, b,
        );
    }

    // --- simd8and16::Cube3 benchmarks ---

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_compose_cube3_simd8and16(b: &mut Bencher) {
        make_guard!(guard);
        bench_compose_helper::<cube3::simd8and16::Cube3>(guard, b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_inverse_cube3_simd8and16(b: &mut Bencher) {
        make_guard!(guard);
        bench_inverse_helper::<cube3::simd8and16::Cube3>(guard, b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_induces_sorted_cycle_type_cube3_simd8and16_worst(b: &mut Bencher) {
        make_guard!(guard);
        bench_induces_sorted_cycle_type_worst_helper::<cube3::simd8and16::Cube3>(guard, b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_induces_sorted_cycle_type_cube3_simd8and16_average(b: &mut Bencher) {
        make_guard!(guard);
        bench_induces_sorted_cycle_type_average_helper::<cube3::simd8and16::Cube3>(guard, b);
    }

    // --- avx2::Cube3 benchmarks ---

    #[bench]
    #[cfg_attr(not(avx2), ignore)]
    fn bench_compose_cube3_avx2(b: &mut Bencher) {
        make_guard!(guard);
        bench_compose_helper::<cube3::avx2::Cube3>(guard, b);
    }

    #[bench]
    #[cfg_attr(not(avx2), ignore)]
    fn bench_inverse_cube3_avx2(b: &mut Bencher) {
        make_guard!(guard);
        bench_inverse_helper::<cube3::avx2::Cube3>(guard, b);
    }

    #[bench]
    #[cfg_attr(not(avx2), ignore)]
    fn bench_induces_sorted_cycle_type_cube3_avx2_worst(b: &mut Bencher) {
        make_guard!(guard);
        bench_induces_sorted_cycle_type_worst_helper::<cube3::avx2::Cube3>(guard, b);
    }

    #[bench]
    #[cfg_attr(not(avx2), ignore)]
    fn bench_induces_sorted_cycle_type_cube3_avx2_average(b: &mut Bencher) {
        make_guard!(guard);
        bench_induces_sorted_cycle_type_average_helper::<cube3::avx2::Cube3>(guard, b);
    }
}
