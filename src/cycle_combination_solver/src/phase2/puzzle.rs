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
    /// Can the implementor be created from these sorted orbit defs? For example
    /// StackPuzzle<39> cannot hold a 3x3 cube state and Cube3Simd cannot hold a
    /// 4x4 cube state
    fn validate_sorted_orbit_defs(
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<(), KSolveConversionError>;
    /// Create a puzzle state from a sorted transformation without checking if
    /// it belongs to orbit defs. Panics if sorted orbit defs are invalid which
    /// is guaranteed to not happen normally by appropriately calling validate
    fn from_sorted_transformations_unchecked(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Self;
    /// Compose two puzzle states in place
    fn replace_compose(&mut self, a: &Self, b: &Self, sorted_orbit_defs: &[OrbitDef]);
    /// Inverse of a puzzle state
    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: &[OrbitDef]);
    /// The goal state for IDA* search
    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        multi_bv: <Self::MultiBv as MultiBvInterface>::MultiBvReusableRef<'_>,
        sorted_orbit_defs: &[OrbitDef],
    ) -> bool;
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

#[derive(Clone, PartialEq, Debug, Hash)]
pub struct StackPuzzle<const N: usize>(pub [u8; N]);

#[derive(Clone, PartialEq, Debug, Hash)]
pub struct HeapPuzzle(pub Box<[u8]>);

pub struct PuzzleDef<P: PuzzleState> {
    pub moves: Vec<Move<P>>,
    pub symmetries: Vec<Move<P>>,
    pub sorted_orbit_defs: Vec<OrbitDef>,
    pub name: String,
}

#[derive(Error, Debug)]
pub enum KSolveConversionError {
    #[error("Phase 2 does not currently support puzzles with set sizes larger than 255, but it will in the future")]
    SetSizeTooBig,
    #[error("Not enough buffer space to convert move")]
    NotEnoughBufferSpace,
    #[error("Could not expand move set, order of a move too high")]
    MoveOrderTooHigh,
    #[error("Invalid KSolve orbit definitions. Expected: {0:?}\nActual: {1:?}")]
    InvalidOrbitDefs(Vec<OrbitDef>, Vec<OrbitDef>),
}

#[derive(Debug, Clone)]
pub struct Move<P: PuzzleState> {
    pub puzzle_state: P,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct OrbitDef {
    pub piece_count: NonZeroU8,
    pub orientation_count: NonZeroU8,
}

pub type OrientedPartition = Vec<(NonZeroU8, bool)>;

impl<P: PuzzleState> PuzzleDef<P> {
    pub fn find_move(&self, name: &str) -> Option<&Move<P>> {
        self.moves.iter().find(|def| def.name == name)
    }

    pub fn find_symmetry(&self, name: &str) -> Option<&Move<P>> {
        self.symmetries.iter().find(|def| def.name == name)
    }

    pub fn solved_state(&self) -> P {
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
    P::from_sorted_transformations_unchecked(&sorted_transformations, sorted_orbit_defs)
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
                // TODO: sort by facelets per piece, distance from center
            )
        });

        sorted_orbit_defs = arg_indicies
            .iter()
            .map(|&i| sorted_orbit_defs[i].clone())
            .collect();

        let mut moves = Vec::with_capacity(ksolve.moves().len());
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

            P::validate_sorted_orbit_defs(sorted_orbit_defs.as_slice())?;
            let puzzle_state = P::from_sorted_transformations_unchecked(
                &sorted_transformations,
                sorted_orbit_defs.as_slice(),
            );

            let base_move = Move {
                name: ksolve_move.name().to_owned(),
                puzzle_state,
            };

            if i >= ksolve.moves().len() {
                symmetries.push(base_move);
                continue;
            }

            let solved: P = solved_state_from_sorted_orbit_defs(&sorted_orbit_defs);
            let mut move_1 = base_move.clone();
            let mut move_2 = base_move.clone();

            let mut move_powers: Vec<P> = vec![];
            const MAX_MOVE_POWER: usize = 1_000_000;

            for _ in 0..MAX_MOVE_POWER {
                move_1.puzzle_state.replace_compose(
                    &move_2.puzzle_state,
                    &base_move.puzzle_state,
                    &sorted_orbit_defs,
                );
                if move_1.puzzle_state == solved {
                    break;
                }
                move_powers.push(move_1.puzzle_state.clone());
                std::mem::swap(&mut move_1, &mut move_2);
            }

            if move_powers.len() == MAX_MOVE_POWER {
                return Err(KSolveConversionError::MoveOrderTooHigh);
            }

            let base_name = base_move.name.clone();
            moves.push(base_move);

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
                    name: expanded_name,
                });
            }
        }

        Ok(PuzzleDef {
            moves,
            symmetries,
            sorted_orbit_defs,
            name: ksolve.name().to_owned(),
        })
    }
}

impl<const N: usize> PuzzleState for StackPuzzle<N> {
    type MultiBv = Box<[u8]>;

    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        new_multi_bv_slice(sorted_orbit_defs)
    }

    fn validate_sorted_orbit_defs(
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<(), KSolveConversionError> {
        if N >= sorted_orbit_defs
            .iter()
            .map(|orbit_def| (orbit_def.piece_count.get() as usize) * 2)
            .sum()
        {
            Ok(())
        } else {
            Err(KSolveConversionError::NotEnoughBufferSpace)
        }
    }

    fn from_sorted_transformations_unchecked(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Self {
        let mut orbit_states = [0_u8; N];
        ksolve_move_to_slice_unchecked(
            &mut orbit_states,
            sorted_orbit_defs,
            sorted_transformations,
        );
        StackPuzzle(orbit_states)
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
        multi_bv: &mut [u8],
        sorted_orbit_defs: &[OrbitDef],
    ) -> bool {
        induces_sorted_cycle_type_slice(&self.0, sorted_cycle_type, multi_bv, sorted_orbit_defs)
    }
}

impl PuzzleState for HeapPuzzle {
    type MultiBv = Box<[u8]>;

    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        new_multi_bv_slice(sorted_orbit_defs)
    }

    fn validate_sorted_orbit_defs(
        _sorted_orbit_defs: &[OrbitDef],
    ) -> Result<(), KSolveConversionError> {
        // No validation needed. from_sorted_transformations_unchecked creates
        // an orbit states buffer that is guaranteed to be the right size, and
        // there is no restriction on the expected orbit defs
        Ok(())
    }

    fn from_sorted_transformations_unchecked(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Self {
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
        HeapPuzzle(orbit_states)
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
        multi_bv: &mut [u8],
        sorted_orbit_defs: &[OrbitDef],
    ) -> bool {
        induces_sorted_cycle_type_slice(&self.0, sorted_cycle_type, multi_bv, sorted_orbit_defs)
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
        for j in 0..piece_count {
            let (perm, orientation_delta) = transformation[j];
            orbit_states[i + j + piece_count] = orientation_delta;
            orbit_states[i + j] = perm;
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
                    *orbit_states_mut.get_unchecked_mut(base + a[base_i] as usize + piece_count) = 0;
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
    multi_bv: &mut [u8],
    sorted_orbit_defs: &[OrbitDef],
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

    fn apply_moves<P: PuzzleState + Clone>(
        puzzle_def: &PuzzleDef<P>,
        puzzle_state: &P,
        moves: &str,
        repeat: u32,
    ) -> P {
        let mut result = puzzle_state.clone();
        let mut prev_result = puzzle_state.clone();

        for _ in 0..repeat {
            for name in moves.split_whitespace() {
                let m = puzzle_def.find_move(name).unwrap();
                prev_result.replace_compose(
                    &result,
                    &m.puzzle_state,
                    &puzzle_def.sorted_orbit_defs,
                );
                std::mem::swap(&mut result, &mut prev_result);
            }
        }
        result
    }

    // TODO: add this test when puzzle geometry is able to generate KSolve
    // puzzles with set sizes larger than 255
    // #[test]
    // fn test_set_size_too_big() {
    //     let cube3_def =
    //     assert!(matches!(
    //         cube3_def,
    //         Err(KSolveConversionError::SetSizeTooBig)
    //     ));
    // }

    #[test]
    fn test_not_enough_buffer_space() {
        let cube3_def = PuzzleDef::<StackPuzzle<39>>::try_from(&*KPUZZLE_3X3);
        assert!(matches!(
            cube3_def,
            Err(KSolveConversionError::NotEnoughBufferSpace)
        ));
    }

    // TODO: add this test when either puzzle geometry exposes another KSolve
    // definition other than KPUZZLE_3X3 or when there is another simd puzzle
    // #[test]
    // fn test_invalid_orbit_defs() {
    //     let cube3_def =
    //     assert!(matches!(
    //         cube3_def,
    //         Err(KSolveConversionError::InvalidOrbitDefs(_, _))
    //     ));
    // }

    fn many_compositions<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.solved_state();
        let also_solved = apply_moves(&cube3_def, &solved, "R F", 105);
        assert_eq!(also_solved, solved);
    }

    #[test]
    fn test_many_compositions() {
        many_compositions::<StackCube3>();
        many_compositions::<HeapPuzzle>();
        many_compositions::<cube3::Cube3>();
    }

    fn s_u4_symmetry<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let s_u4_symmetry = cube3_def.find_symmetry("S_U4").unwrap();

        let mut result = cube3_def.solved_state();
        let mut prev_result = result.clone();
        for _ in 0..4 {
            prev_result.replace_compose(
                &result,
                &s_u4_symmetry.puzzle_state,
                &cube3_def.sorted_orbit_defs,
            );
            std::mem::swap(&mut result, &mut prev_result);
        }

        let solved = cube3_def.solved_state();
        assert_eq!(result, solved);
    }

    #[test]
    fn test_s_u4_symmetry() {
        s_u4_symmetry::<StackCube3>();
        s_u4_symmetry::<HeapPuzzle>();
        s_u4_symmetry::<cube3::Cube3>();
    }

    fn expanded_move<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let actual_solved = cube3_def.solved_state();
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
        expanded_move::<cube3::Cube3>();
    }

    fn inversion<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.solved_state();
        let mut result = cube3_def.solved_state();

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
        inversion::<cube3::Cube3>();
    }

    fn random_inversion<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.solved_state();

        for _ in 0..100 {
            let mut prev_result = cube3_def.solved_state();
            let mut result = cube3_def.solved_state();
            for _ in 0..20 {
                let move_index = fastrand::choice(0_u8..18).unwrap();
                let move_ = &cube3_def.moves[move_index as usize];
                prev_result.replace_compose(
                    &result,
                    &move_.puzzle_state,
                    &cube3_def.sorted_orbit_defs,
                );
                std::mem::swap(&mut result, &mut prev_result);
            }
            prev_result.replace_inverse(&result, &cube3_def.sorted_orbit_defs);
            result.replace_compose(&prev_result, &result.clone(), &cube3_def.sorted_orbit_defs);
            assert_eq!(result, solved);
        }
    }

    #[test]
    fn test_random_inversion() {
        random_inversion::<StackCube3>();
        random_inversion::<HeapPuzzle>();
        random_inversion::<cube3::Cube3>();
    }

    fn induces_sorted_cycle_type_within_cycle<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.solved_state();
        let mut multi_bv = P::new_multi_bv(&cube3_def.sorted_orbit_defs);

        let order_1260 = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 1);
        let sorted_cycle_type = [
            ct(&[(3, true), (5, true)]),
            ct(&[(2, true), (2, false), (7, true)]),
        ];
        assert!(order_1260.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs,
        ));

        let order_1260_in_cycle = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 209);
        assert!(order_1260_in_cycle.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
    }

    #[test]
    fn test_induces_sorted_cycle_type_within_cycle() {
        induces_sorted_cycle_type_within_cycle::<StackCube3>();
        induces_sorted_cycle_type_within_cycle::<HeapPuzzle>();
        induces_sorted_cycle_type_within_cycle::<cube3::Cube3>();
    }

    fn induces_sorted_cycle_type_many<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.solved_state();
        let mut multi_bv = P::new_multi_bv(&cube3_def.sorted_orbit_defs);
        assert!(solved.induces_sorted_cycle_type(
            &[vec![], vec![]],
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
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
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
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
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
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
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
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
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
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
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
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
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
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
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
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
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
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
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
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
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));

        let random_state = apply_moves(&cube3_def, &solved, "U L U L2 U2 B2", 1);
        let sorted_cycle_type = [
            ct(&[(1, true), (2, false), (3, true)]),
            ct(&[(2, false), (3, false), (3, false)]),
        ];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));

        let random_state = apply_moves(&cube3_def, &solved, "U", 1);
        let sorted_cycle_type = [ct(&[(4, false)]), ct(&[(4, false)])];
        assert!(random_state.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
        assert!(!solved.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs
        ));
    }

    #[test]
    fn test_induces_sorted_cycle_type_many() {
        induces_sorted_cycle_type_many::<StackCube3>();
        induces_sorted_cycle_type_many::<HeapPuzzle>();
        induces_sorted_cycle_type_many::<cube3::Cube3>();
    }

    fn bench_induces_sorted_cycle_type_helper<P: PuzzleState>(b: &mut Bencher) {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let sorted_cycle_type = [
            ct(&[(3, true), (5, true)]),
            ct(&[(2, true), (2, false), (7, true)]),
        ];
        let order_1260 = apply_moves(&cube3_def, &cube3_def.solved_state(), "R U2 D' B D'", 1);
        let mut multi_bv = P::new_multi_bv(&cube3_def.sorted_orbit_defs);
        b.iter(|| {
            test::black_box(&order_1260).induces_sorted_cycle_type(
                &sorted_cycle_type,
                multi_bv.reusable_ref(),
                &cube3_def.sorted_orbit_defs,
            );
        });
    }

    fn bench_inverse_puzzle_helper<P: PuzzleState>(b: &mut Bencher) {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.solved_state();
        let mut result = solved.clone();
        let order_1260 = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 100);
        b.iter(|| {
            test::black_box(&mut result)
                .replace_inverse(test::black_box(&order_1260), &cube3_def.sorted_orbit_defs);
        });
    }

    fn bench_compose_puzzle_helper<P: PuzzleState>(b: &mut Bencher) {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut solved = cube3_def.solved_state();
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

    #[bench]
    fn bench_compose_stack(b: &mut Bencher) {
        bench_compose_puzzle_helper::<StackCube3>(b);
    }

    #[bench]
    fn bench_compose_heap(b: &mut Bencher) {
        bench_compose_puzzle_helper::<HeapPuzzle>(b);
    }

    #[bench]
    fn bench_compose_cube3(b: &mut Bencher) {
        bench_compose_puzzle_helper::<cube3::Cube3>(b);
    }

    #[bench]
    fn bench_inverse_stack(b: &mut Bencher) {
        bench_inverse_puzzle_helper::<StackCube3>(b);
    }

    #[bench]
    fn bench_inverse_heap(b: &mut Bencher) {
        bench_inverse_puzzle_helper::<HeapPuzzle>(b);
    }

    #[bench]
    fn bench_inverse_cube3(b: &mut Bencher) {
        bench_inverse_puzzle_helper::<cube3::Cube3>(b);
    }

    #[bench]
    fn bench_induces_sorted_cycle_type_stack(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_helper::<StackCube3>(b);
    }

    #[bench]
    fn bench_induces_sorted_cycle_type_heap(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_helper::<HeapPuzzle>(b);
    }

    #[bench]
    fn bench_induces_sorted_cycle_type_cube3(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_helper::<cube3::Cube3>(b);
    }
}
