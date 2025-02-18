use num_traits::PrimInt;
use puzzle_geometry::ksolve::KSolve;
use std::{fmt::Debug, num::NonZeroU8};
use thiserror::Error;

pub mod cube3;

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

pub trait PuzzleState
where
    Self: Sized + Clone + PartialEq + Debug,
{
    type MultiBv: MultiBvInterface;

    /// Get a default multi bit vector for use in `induces_sorted_cycle_type`
    fn default_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv;
    /// Get the implmentor's orbit definition specification, or None if any
    /// orbit definition is allowed
    fn expected_sorted_orbit_defs() -> Option<&'static [OrbitDef]>;
    /// Create a puzzle state from a sorted transformation without checking if
    /// it belongs to orbit defs
    fn from_sorted_transformations_unchecked(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError>;
    /// Compose two puzzle states in place
    fn replace_compose(&mut self, a: &Self, b: &Self, sorted_orbit_defs: &[OrbitDef]);
    /// The goal state for IDA* search
    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        multi_bv: <Self::MultiBv as MultiBvInterface>::MultiBvReusableRef<'_>,
        sorted_orbit_defs: &[OrbitDef],
    ) -> bool;
}

#[derive(Clone, PartialEq, Debug)]
pub struct StackPuzzle<const N: usize>(pub [u8; N]);

#[derive(Clone, PartialEq, Debug)]
pub struct HeapPuzzle(pub Box<[u8]>);

pub type StackCube3 = StackPuzzle<40>;

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

    pub fn solved_state(&self) -> Result<P, KSolveConversionError> {
        solved_state_from_sorted_orbit_defs(&self.sorted_orbit_defs)
    }
}

fn solved_state_from_sorted_orbit_defs<P: PuzzleState>(
    sorted_orbit_defs: &[OrbitDef],
) -> Result<P, KSolveConversionError> {
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
        sorted_orbit_defs
            .sort_by_key(|orbit_def| (orbit_def.piece_count, orbit_def.orientation_count));

        if let Some(expected_sorted_orbit_defs) = P::expected_sorted_orbit_defs() {
            if sorted_orbit_defs != expected_sorted_orbit_defs {
                return Err(KSolveConversionError::InvalidOrbitDefs(
                    expected_sorted_orbit_defs.to_vec(),
                    sorted_orbit_defs,
                ));
            }
        }

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
            // TODO: sorting should take orientation_count into account
            sorted_transformations.sort_by_key(|a| a.len());

            let puzzle_state = P::from_sorted_transformations_unchecked(
                &sorted_transformations,
                sorted_orbit_defs.as_slice(),
            )?;

            let base_move = Move {
                name: ksolve_move.name().to_owned(),
                puzzle_state,
            };

            if i >= ksolve.moves().len() {
                symmetries.push(base_move);
                continue;
            }

            let solved: P = solved_state_from_sorted_orbit_defs(&sorted_orbit_defs)?;
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

    fn default_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        default_multi_bv_slice(sorted_orbit_defs)
    }

    fn expected_sorted_orbit_defs() -> Option<&'static [OrbitDef]> {
        None
    }

    fn from_sorted_transformations_unchecked(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError> {
        let mut orbit_states = [0_u8; N];
        ksolve_move_to_slice_unchecked(
            &mut orbit_states,
            sorted_orbit_defs,
            sorted_transformations,
        )?;
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

    fn default_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        default_multi_bv_slice(sorted_orbit_defs)
    }

    fn expected_sorted_orbit_defs() -> Option<&'static [OrbitDef]> {
        None
    }

    fn from_sorted_transformations_unchecked(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError> {
        let mut orbit_states = vec![
            0_u8;
            sorted_transformations
                .iter()
                .map(|perm_and_ori| perm_and_ori.len() * 2)
                .sum()
        ]
        .into_boxed_slice();
        ksolve_move_to_slice_unchecked(
            &mut orbit_states,
            sorted_orbit_defs,
            sorted_transformations,
        )?;
        Ok(HeapPuzzle(orbit_states))
    }

    fn replace_compose(&mut self, a: &HeapPuzzle, b: &HeapPuzzle, sorted_orbit_defs: &[OrbitDef]) {
        replace_compose_slice(&mut self.0, &a.0, &b.0, sorted_orbit_defs);
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

fn default_multi_bv_slice(sorted_orbit_defs: &[OrbitDef]) -> Box<[u8]> {
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
) -> Result<(), KSolveConversionError> {
    let mut i = 0;
    for (transformation, orbit_def) in sorted_transformations.iter().zip(sorted_orbit_defs.iter()) {
        let piece_count = orbit_def.piece_count.get() as usize;
        for j in 0..piece_count {
            let (perm, orientation_delta) = transformation[j];
            *orbit_states
                .get_mut(i + j + piece_count)
                .ok_or(KSolveConversionError::NotEnoughBufferSpace)? = orientation_delta;
            orbit_states[i + j] = perm;
        }
        i += piece_count * 2;
    }
    Ok(())
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
        // TODO: add back get_unchecked_mut etc and for other slice stuff too
        // it's 60% faster surprisingly
        if orientation_count == 1.try_into().unwrap() {
            for i in 0..piece_count {
                orbit_states_mut[base + i] = a[base + b[base + i] as usize];
                orbit_states_mut[base + i + piece_count] = 0;
            }
        } else {
            for i in 0..piece_count {
                orbit_states_mut[base + i] = a[base + b[base + i] as usize];
                orbit_states_mut[base + i + piece_count] =
                    (a[base + b[base + i] as usize + piece_count] + b[base + i + piece_count])
                        % orientation_count;
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
            if multi_bv[div] & (1 << rem) != 0 {
                continue;
            }
            multi_bv[div] |= 1 << rem;
            let mut actual_cycle_length = 1;
            let mut piece = orbit_states[base + i] as usize;
            let mut orientation_sum = orbit_states[base + piece + piece_count];

            while piece != i {
                actual_cycle_length += 1;
                let (div, rem) = (piece / 4, piece % 4);
                multi_bv[div] |= 1 << rem;
                piece = orbit_states[base + piece] as usize;
                orientation_sum += orbit_states[base + piece + piece_count];
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
                        && (multi_bv[div] & (1 << (rem + 4)) == 0)
                },
            ) else {
                return false;
            };
            let (div, rem) = (valid_cycle_index / 4, valid_cycle_index % 4);
            multi_bv[div] |= 1 << (rem + 4);
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

    fn apply_moves<P: PuzzleState + Clone>(
        puzzle_def: &PuzzleDef<P>,
        puzzle_state: P,
        moves: &str,
    ) -> P {
        let mut result = puzzle_state.clone();
        let mut prev_result = puzzle_state.clone();
        for name in moves.split_whitespace() {
            let m = puzzle_def.find_move(name).unwrap();
            prev_result.replace_compose(&result, &m.puzzle_state, &puzzle_def.sorted_orbit_defs);
            std::mem::swap(&mut result, &mut prev_result);
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
        let solved = cube3_def.solved_state().unwrap();
        let also_solved = apply_moves(&cube3_def, solved.clone(), &"R F ".repeat(105));
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

        let mut result = cube3_def.solved_state().unwrap();
        let mut prev_result = result.clone();
        for _ in 0..4 {
            prev_result.replace_compose(
                &result,
                &s_u4_symmetry.puzzle_state,
                &cube3_def.sorted_orbit_defs,
            );
            std::mem::swap(&mut result, &mut prev_result);
        }

        let solved = cube3_def.solved_state().unwrap();
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
        let actual_solved = cube3_def.solved_state().unwrap();
        let expected_solved = apply_moves(
            &cube3_def,
            actual_solved.clone(),
            "R R' D2 D2 U U U2 F B' F' B",
        );
        assert_eq!(actual_solved, expected_solved);
    }

    #[test]
    fn test_expanded_move() {
        expanded_move::<StackCube3>();
        expanded_move::<HeapPuzzle>();
        expanded_move::<cube3::Cube3>();
    }

    fn induces_sorted_cycle_type<P: PuzzleState>() {
        let cube3_def: PuzzleDef<P> = (&*KPUZZLE_3X3).try_into().unwrap();
        let order_1260 = apply_moves(
            &cube3_def,
            cube3_def.solved_state().unwrap(),
            "R U2 D' B D'",
        );
        let sorted_cycle_type = vec![
            vec![(3.try_into().unwrap(), true), (5.try_into().unwrap(), true)],
            vec![
                (2.try_into().unwrap(), true),
                (2.try_into().unwrap(), false),
                (7.try_into().unwrap(), true),
            ],
        ];
        let mut multi_bv = P::default_multi_bv(&cube3_def.sorted_orbit_defs);
        assert!(order_1260.induces_sorted_cycle_type(
            &sorted_cycle_type,
            multi_bv.reusable_ref(),
            &cube3_def.sorted_orbit_defs,
        ));
    }

    #[test]
    fn test_induces_sorted_cycle_type() {
        induces_sorted_cycle_type::<StackCube3>();
        induces_sorted_cycle_type::<HeapPuzzle>();
        induces_sorted_cycle_type::<cube3::Cube3>();
    }

    // TODO: bench induces cycle type

    #[bench]
    fn bench_compose_stack(b: &mut Bencher) {
        let cube3_def: PuzzleDef<StackCube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut solved = cube3_def.solved_state().unwrap();
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
    fn bench_compose_heap(b: &mut Bencher) {
        let cube3_def: PuzzleDef<HeapPuzzle> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut solved = cube3_def.solved_state().unwrap();
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
    fn bench_compose_simd(b: &mut Bencher) {
        let cube3_def: PuzzleDef<cube3::Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut solved = cube3_def.solved_state().unwrap();
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
}
