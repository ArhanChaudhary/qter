use itertools::Itertools;
use num_traits::PrimInt;
use puzzle_geometry::ksolve::KSolve;
use std::{fmt::Debug, hash::Hash, num::NonZeroU8};
use thiserror::Error;

pub mod cube3;
pub mod slice_puzzle;

/// The puzzle state interface at the heart of the cycle combination solver.
/// Users may either use the generic `HeapPuzzle` implementor for any `KSolve`
/// definition or define fast puzzle-specific implementations, like Cube3.
pub trait PuzzleState: Clone + PartialEq + Debug {
    /// A reusable multi bit vector type to hold temporary storage in
    /// `induces_sorted_cycle_type`.
    type MultiBv: MultiBvInterface;
    type OrbitBytesBuf<'a>: AsRef<[u8]>
    where
        Self: 'a;

    /// Get a default multi bit vector for use in `induces_sorted_cycle_type`
    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv;

    /// Create a puzzle state from a sorted transformation and sorted
    /// orbit defs. `sorted_transformations` must to correspond to
    /// `sorted_orbit_defs`.
    ///
    /// # Errors
    ///
    /// If a puzzle state cannot be created from the orbit
    fn try_from_transformation_meta(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError>;

    /// Compose two puzzle states in place
    ///
    /// # Safety
    ///
    /// `a` and `b` must both correspond to `sorted_orbit_defs`.
    unsafe fn replace_compose(&mut self, a: &Self, b: &Self, sorted_orbit_defs: &[OrbitDef]);

    /// Inverse of a puzzle state
    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: &[OrbitDef]);

    /// The goal state for IDA* search
    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: <Self::MultiBv as MultiBvInterface>::ReusableRef<'_>,
    ) -> bool;

    /// Get a usize that "identifies" an orbit. This is implementor-specific.
    /// For slice puzzles, the identifier is the starting index of the orbit data
    /// in the puzzle state buffer. For specific puzzles the identifier is the
    /// index of the orbit in the orbit definition.
    fn next_orbit_identifer(orbit_identifier: usize, orbit_def: OrbitDef) -> usize;

    /// Get the bytes of the specified orbit index in the form (permutation
    /// vector, orientation vector).
    fn orbit_bytes(
        &self,
        orbit_identifier: usize,
        orbit_def: OrbitDef,
    ) -> (Self::OrbitBytesBuf<'_>, Self::OrbitBytesBuf<'_>);

    /// Return an integer that corresponds to a bijective mapping of the orbit
    /// identifier's states.
    fn exact_hasher_orbit(&self, orbit_identifier: usize, orbit_def: OrbitDef) -> u64;

    /// Return a representation of the puzzle state that can be soundly hashed.
    fn approximate_hash_orbit(&self, orbit_identifier: usize, orbit_def: OrbitDef) -> impl Hash;
}

pub trait MultiBvInterface {
    type ReusableRef<'a>
    where
        Self: 'a;

    fn reusable_ref(&mut self) -> Self::ReusableRef<'_>;
}

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
    #[error("Invalid KSolve orbit definitions. Expected: {expected:?}\nActual: {actual:?}")]
    InvalidOrbitDefs {
        expected: Vec<OrbitDef>,
        actual: Vec<OrbitDef>,
    },
}

#[derive(Debug, Clone)]
pub struct Move<P: PuzzleState> {
    pub puzzle_state: P,
    pub move_class_index: usize,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct OrbitDef {
    pub piece_count: NonZeroU8,
    pub orientation_count: NonZeroU8,
}

pub type OrientedPartition = Vec<(NonZeroU8, bool)>;

#[non_exhaustive]
pub struct TransformationMeta<'a> {
    pub sorted_transformation: Vec<(u8, u8)>,
    pub sorted_orbit_defs: &'a [OrbitDef],
}

impl<'a> TransformationMeta<'a> {
    #[must_use]
    pub fn new(sorted_transformation: Vec<(u8, u8)>, sorted_orbit_defs: &'a [OrbitDef]) -> Self {
        Self {
            sorted_transformation,
            sorted_orbit_defs,
        }
    }
}

impl<P: PuzzleState> Move<P> {
    /// # Safety
    ///
    /// `self` and `other` must both correspond to `sorted_orbit_defs`.
    pub unsafe fn commutes_with(
        &self,
        other: &Self,
        result_1: &mut P,
        result_2: &mut P,
        sorted_orbit_defs: &[OrbitDef],
    ) -> bool {
        // SAFETY: the caller guarantees that `self` and `other` correspond to
        // `sorted_orbit_defs`
        unsafe {
            result_1.replace_compose(&self.puzzle_state, &other.puzzle_state, sorted_orbit_defs);
            result_2.replace_compose(&other.puzzle_state, &self.puzzle_state, sorted_orbit_defs);
        }
        result_1 == result_2
    }
}

impl<P: PuzzleState> PuzzleDef<P> {
    #[must_use]
    pub fn find_move(&self, name: &str) -> Option<&Move<P>> {
        self.moves.iter().find(|move_| move_.name == name)
    }

    #[must_use]
    pub fn find_symmetry(&self, name: &str) -> Option<&Move<P>> {
        self.symmetries.iter().find(|move_| move_.name == name)
    }

    #[must_use]
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
                .collect_vec()
        })
        .collect_vec();
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

        let mut arg_indicies = (0..sorted_orbit_defs.len()).collect_vec();
        arg_indicies.sort_by_key(|&i| {
            (
                sorted_orbit_defs[i].piece_count.get(),
                sorted_orbit_defs[i].orientation_count.get(),
            )
        });

        sorted_orbit_defs = arg_indicies.iter().map(|&i| sorted_orbit_defs[i]).collect();

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
                .map(|perm_and_ori| {
                    perm_and_ori
                        .iter()
                        .map(|&(perm, orientation)| {
                            // we can unwrap because sorted_orbit_defs exists
                            ((perm.get() - 1).try_into().unwrap(), orientation)
                        })
                        .collect_vec()
                })
                .collect_vec();
            sorted_transformations = arg_indicies
                .iter()
                .map(|&i| sorted_transformations[i].clone())
                .collect();

            let puzzle_state =
                P::try_from_transformation_meta(&sorted_transformations, &sorted_orbit_defs)?;

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
            for _ in 0..MAX_MOVE_POWER {
                // SAFETY: the arguments correspond to `sorted_orbit_defs`
                unsafe {
                    result_1.replace_compose(
                        &result_2,
                        &base_move.puzzle_state,
                        &sorted_orbit_defs,
                    );
                }
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
            let order = isize::try_from(move_powers.len()).unwrap() + 2;
            for (j, expanded_puzzle_state) in move_powers.into_iter().enumerate() {
                // see above
                let mut twist = isize::try_from(j).unwrap() + 2;
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

impl MultiBvInterface for () {
    type ReusableRef<'a> = ();

    fn reusable_ref(&mut self) -> Self::ReusableRef<'_> {}
}

/// A utility function for testing. Not optimized.
///
/// # Panics
///
/// Panics if the move sequence is invalid.
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
            // SAFETY: the arguments correspond to `sorted_orbit_defs`
            unsafe {
                result_2.replace_compose(
                    &result_1,
                    &move_.puzzle_state,
                    &puzzle_def.sorted_orbit_defs,
                );
            }
            std::mem::swap(&mut result_1, &mut result_2);
        }
    }
    result_1
}

#[cfg(test)]
mod tests {
    extern crate test;

    use super::{
        slice_puzzle::{HeapPuzzle, StackPuzzle},
        *,
    };
    use crate::phase2::orbit_puzzle::cube3::random_3x3_state;
    use puzzle_geometry::ksolve::KPUZZLE_3X3;
    use test::Bencher;

    type StackCube3 = StackPuzzle<40>;

    fn ct(sorted_cycle_type: &[(u8, bool)]) -> OrientedPartition {
        sorted_cycle_type
            .iter()
            .map(|&(length, oriented)| (length.try_into().unwrap(), oriented))
            .collect()
    }

    fn commutes_with<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut result_1 = cube3_def.new_solved_state();
        let mut result_2 = result_1.clone();

        let u_move = cube3_def.find_move("U").unwrap();
        let d2_move = cube3_def.find_move("D2").unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        unsafe {
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
    }

    #[test]
    fn test_commutes_with() {
        commutes_with::<StackCube3>();
        commutes_with::<HeapPuzzle>();
        #[cfg(simd8and16)]
        commutes_with::<cube3::simd8and16::Cube3>();
        #[cfg(simd8and16)]
        commutes_with::<cube3::simd8and16::UncompressedCube3>();
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
        #[cfg(simd8and16)]
        many_compositions::<cube3::simd8and16::UncompressedCube3>();
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
            unsafe {
                result_2.replace_compose(
                    &result_1,
                    &s_u4_symmetry.puzzle_state,
                    &cube3_def.sorted_orbit_defs,
                );
            }
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
        #[cfg(simd8and16)]
        s_u4_symmetry::<cube3::simd8and16::UncompressedCube3>();
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
        #[cfg(simd8and16)]
        expanded_move::<cube3::simd8and16::UncompressedCube3>();
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
            println!("{result:?}\n{remaining_state:?}\n\n");
            assert_eq!(result, remaining_state);
        }
    }

    #[test]
    fn test_inversion() {
        inversion::<StackCube3>();
        inversion::<HeapPuzzle>();
        #[cfg(simd8and16)]
        inversion::<cube3::simd8and16::Cube3>();
        #[cfg(simd8and16)]
        inversion::<cube3::simd8and16::UncompressedCube3>();
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
            unsafe {
                result_2.replace_compose(&result_1, &random_state, &cube3_def.sorted_orbit_defs);
            }

            assert_eq!(result_2, solved);
        }
    }

    #[test]
    fn test_random_inversion() {
        random_inversion::<StackCube3>();
        random_inversion::<HeapPuzzle>();
        #[cfg(simd8and16)]
        random_inversion::<cube3::simd8and16::Cube3>();
        #[cfg(simd8and16)]
        random_inversion::<cube3::simd8and16::UncompressedCube3>();
        #[cfg(avx2)]
        random_inversion::<cube3::avx2::Cube3>();
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
        #[cfg(simd8and16)]
        induces_sorted_cycle_type_within_cycle::<cube3::simd8and16::UncompressedCube3>();
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

        let tests = [
            (
                "F2 L' U2 F U F U L' B U' F' U D2 L F2 B'",
                &[ct(&[(1, true), (3, true)]), ct(&[(1, true), (5, true)])],
            ),
            (
                "U2 L B L2 F U2 B' U2 R U' F R' F' R F' L' U2",
                &[ct(&[(1, true), (5, true)]), ct(&[(1, true), (7, true)])],
            ),
            (
                "R' U2 R' U2 F' D' L F L2 F U2 F2 D' L' D2 F R2",
                &[ct(&[(1, true), (3, true)]), ct(&[(1, true), (7, true)])],
            ),
            (
                "B2 U' B' D B' L' D' B U' R2 B2 R U B2 R B' R U",
                &[
                    ct(&[(1, true), (1, true), (3, true)]),
                    ct(&[(1, true), (7, true)]),
                ],
            ),
            (
                "R2 L2 D' B L2 D' B L' B D2 R2 B2 R' D' B2 L2 U'",
                &[ct(&[(2, true), (3, true)]), ct(&[(4, true), (5, true)])],
            ),
            (
                "F' B2 R L U2 B U2 L2 F2 U R L B' L' D' R' D' B'",
                &[
                    ct(&[(1, true), (2, true), (3, true)]),
                    ct(&[(4, true), (5, true)]),
                ],
            ),
            (
                "L' D2 F B2 U F' L2 B R F2 D R' L F R' F' D",
                &[
                    ct(&[(2, true), (3, true)]),
                    ct(&[(1, true), (4, true), (5, false)]),
                ],
            ),
            (
                "B' L' F2 R U' R2 F' L2 F R' L B L' U' F2 U' D2 L",
                &[
                    ct(&[(1, true), (2, true), (3, true)]),
                    ct(&[(1, true), (4, true), (5, false)]),
                ],
            ),
            (
                "F2 D2 L' F D R2 F2 U2 L2 F R' B2 D2 R2 U R2 U",
                &[
                    ct(&[(1, true), (2, false), (3, true)]),
                    ct(&[(4, true), (5, true)]),
                ],
            ),
            (
                "F2 B' R' F' L' D B' U' F U B' U2 D L' F' L' B R2",
                &[
                    ct(&[(1, true), (2, false), (3, true)]),
                    ct(&[(1, true), (4, true), (5, false)]),
                ],
            ),
            (
                "U L U L2 U2 B2",
                &[
                    ct(&[(1, true), (2, false), (3, true)]),
                    ct(&[(2, false), (3, false), (3, false)]),
                ],
            ),
            ("U", &[ct(&[(4, false)]), ct(&[(4, false)])]),
        ];

        // for (moves_str, expected_cts) in tests {
        for (i, &(moves_str, expected_cts)) in tests.iter().enumerate() {
            let random_state = apply_moves(&cube3_def, &solved, moves_str, 1);

            assert!(random_state.induces_sorted_cycle_type(
                expected_cts,
                &cube3_def.sorted_orbit_defs,
                multi_bv.reusable_ref(),
            ));

            assert!(!solved.induces_sorted_cycle_type(
                expected_cts,
                &cube3_def.sorted_orbit_defs,
                multi_bv.reusable_ref(),
            ));

            for (j, &(other_moves, _)) in tests.iter().enumerate() {
                if i == j {
                    continue;
                }
                let other_state = apply_moves(&cube3_def, &solved, other_moves, 1);
                assert!(!other_state.induces_sorted_cycle_type(
                    expected_cts,
                    &cube3_def.sorted_orbit_defs,
                    multi_bv.reusable_ref()
                ));
            }
        }
    }

    #[test]
    fn test_induces_sorted_cycle_type_many() {
        induces_sorted_cycle_type_many::<StackCube3>();
        induces_sorted_cycle_type_many::<HeapPuzzle>();
        #[cfg(simd8and16)]
        induces_sorted_cycle_type_many::<cube3::simd8and16::Cube3>();
        #[cfg(simd8and16)]
        induces_sorted_cycle_type_many::<cube3::simd8and16::UncompressedCube3>();
        #[cfg(avx2)]
        induces_sorted_cycle_type_many::<cube3::avx2::Cube3>();
    }

    fn exact_hasher_orbit<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
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
            let mut orbit_identifier = 0;
            for (i, &orbit_def) in cube3_def.sorted_orbit_defs.iter().enumerate() {
                let hash = test_state.exact_hasher_orbit(orbit_identifier, orbit_def);
                assert_eq!(hash, exp_hashes[i]);
                orbit_identifier = P::next_orbit_identifer(orbit_identifier, orbit_def);
            }
        }
    }

    #[test]
    fn test_exact_hasher_orbit() {
        exact_hasher_orbit::<StackCube3>();
        exact_hasher_orbit::<HeapPuzzle>();
        #[cfg(simd8and16)]
        many_compositions::<cube3::simd8and16::Cube3>();
        #[cfg(simd8and16)]
        exact_hasher_orbit::<cube3::simd8and16::UncompressedCube3>();
        #[cfg(avx2)]
        exact_hasher_orbit::<cube3::avx2::Cube3>();
    }

    pub fn bench_compose_helper<P: PuzzleState>(b: &mut Bencher) {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut solved = cube3_def.new_solved_state();
        let r_move = cube3_def.find_move("R").unwrap();
        let f_move = cube3_def.find_move("F").unwrap();
        b.iter(|| unsafe {
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

    // --- HeapPuzzle benchmarks ---

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

    // --- simd8and16::UncompressedCube3 benchmarks ---

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_compose_uncompressed_cube3_simd8and16(b: &mut Bencher) {
        bench_compose_helper::<cube3::simd8and16::UncompressedCube3>(b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_inverse_uncompressed_cube3_simd8and16(b: &mut Bencher) {
        bench_inverse_helper::<cube3::simd8and16::UncompressedCube3>(b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_induces_sorted_cycle_type_uncompressed_cube3_simd8and16_worst(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_worst_helper::<cube3::simd8and16::UncompressedCube3>(b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_induces_sorted_cycle_type_uncompressed_cube3_simd8and16_average(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_average_helper::<cube3::simd8and16::UncompressedCube3>(b);
    }

    // --- simd8and16::Cube3 benchmarks ---

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

    // --- avx2::Cube3 benchmarks ---

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
}
