use puzzle_geometry::ksolve::KSolve;
use std::{
    num::NonZeroU8,
    simd::{u8x16, u8x8},
    sync::LazyLock,
};
use thiserror::Error;

pub trait PuzzleState
where
    Self: Sized,
{
    type ReplaceComposeMeta;

    /// Compose two puzzle states in place
    fn replace_compose(
        &mut self,
        a: &Self,
        b: &Self,
        replace_compose_meta: &Self::ReplaceComposeMeta,
    );
    /// Get the implmentor's orbit definition specification, or None if any
    /// orbit definition is allowed
    fn expected_sorted_orbit_defs() -> Option<&'static [OrbitDef]>;
    /// Create a puzzle state from a ksolve move without checking if the move is
    /// part of the original KSolve
    fn from_sorted_transformations_unchecked(
        sorted_orbit_defs: &[OrbitDef],
        sorted_transformations: &[Vec<(u8, u8)>],
    ) -> Result<Self, KSolveConversionError>;
}

pub struct StackPuzzle<const N: usize>([u8; N]);
pub struct HeapPuzzle(Box<[u8]>);

pub struct PuzzleDef<P: PuzzleState> {
    pub moves: Vec<Move<P>>,
    pub sorted_orbit_defs: Vec<OrbitDef>,
    pub name: String,
}

pub struct Move<P: PuzzleState> {
    pub puzzle_state: P,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct OrbitDef {
    pub piece_count: NonZeroU8,
    pub orientation_count: NonZeroU8,
}

#[derive(Error, Debug)]
pub enum KSolveConversionError {
    #[error("Phase 2 does not currently support puzzles with set sizes larger than 255, but it will in the future")]
    SetSizeTooBig,
    #[error("Not enough buffer space to convert move")]
    NotEnoughBufferSpace,
    #[error("Invalid KSolve orbit definitions. Expected: {0:?}\nActual: {1:?}")]
    InvalidOrbitDefs(Vec<OrbitDef>, Vec<OrbitDef>),
}

impl<const N: usize> PuzzleState for StackPuzzle<N> {
    type ReplaceComposeMeta = Vec<OrbitDef>;

    fn replace_compose(
        &mut self,
        a: &StackPuzzle<N>,
        b: &StackPuzzle<N>,
        puzzle_meta: &Vec<OrbitDef>,
    ) {
        slice_replace_compose(&mut self.0, &a.0, &b.0, puzzle_meta);
    }

    fn expected_sorted_orbit_defs() -> Option<&'static [OrbitDef]> {
        None
    }

    fn from_sorted_transformations_unchecked(
        sorted_orbit_defs: &[OrbitDef],
        sorted_transformations: &[Vec<(u8, u8)>],
    ) -> Result<Self, KSolveConversionError> {
        let mut orbit_states = [0_u8; N];
        ksolve_move_to_slice_unchecked(
            &mut orbit_states,
            sorted_orbit_defs,
            sorted_transformations,
        )?;
        Ok(StackPuzzle(orbit_states))
    }
}

impl PuzzleState for HeapPuzzle {
    type ReplaceComposeMeta = Vec<OrbitDef>;

    fn replace_compose(
        &mut self,
        a: &HeapPuzzle,
        b: &HeapPuzzle,
        replace_compose_meta: &Vec<OrbitDef>,
    ) {
        slice_replace_compose(&mut self.0, &a.0, &b.0, replace_compose_meta);
    }

    fn expected_sorted_orbit_defs() -> Option<&'static [OrbitDef]> {
        None
    }

    fn from_sorted_transformations_unchecked(
        sorted_orbit_defs: &[OrbitDef],
        sorted_transformations: &[Vec<(u8, u8)>],
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
}

fn slice_replace_compose(
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

impl<P: PuzzleState> PuzzleDef<P> {
    pub fn find_move(&self, name: &str) -> Option<&Move<P>> {
        self.moves.iter().find(|def| def.name == name)
    }

    pub fn solved_state(&self) -> Result<P, KSolveConversionError> {
        let sorted_transformations = self
            .sorted_orbit_defs
            .iter()
            .map(|orbit_def| {
                (0..orbit_def.piece_count.get())
                    .map(|i| (i, 0))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        P::from_sorted_transformations_unchecked(&self.sorted_orbit_defs, &sorted_transformations)
    }
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
        sorted_orbit_defs.sort_by_key(|orbit_def| orbit_def.piece_count);

        if let Some(expected_orbit_defs) = P::expected_sorted_orbit_defs() {
            if sorted_orbit_defs != expected_orbit_defs {
                return Err(KSolveConversionError::InvalidOrbitDefs(
                    expected_orbit_defs.to_vec(),
                    sorted_orbit_defs,
                ));
            }
        }

        let moves: Vec<Move<P>> = ksolve
            .moves()
            .iter()
            .map(|ksolve_move| {
                let mut sorted_transformations = ksolve_move
                    .transformation()
                    .iter()
                    .map(|perm_and_ori| {
                        perm_and_ori
                            .iter()
                            .map(|&(perm, orientation)| {
                                // we can unwrap because sorted_orbit_defs
                                // executed without error
                                ((perm.get() - 1).try_into().unwrap(), orientation)
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                sorted_transformations.sort_by_key(|a| a.len());
                let puzzle_state = P::from_sorted_transformations_unchecked(
                    &sorted_orbit_defs,
                    &sorted_transformations,
                )?;
                // TODO: validate here!
                Ok(Move {
                    name: ksolve_move.name().to_owned(),
                    puzzle_state,
                })
            })
            .collect::<Result<_, KSolveConversionError>>()?;

        Ok(PuzzleDef {
            moves,
            sorted_orbit_defs,
            name: ksolve.name().to_owned(),
        })
    }
}

// TODO: Utilize #[cfg(simd8)] #[cfg(simd16)] and #[cfg(simd32)] for differing
// implementations
pub struct StackCube3Simd {
    pub ep: u8x16,
    pub eo: u8x16,
    pub cp: u8x8,
    pub co: u8x8,
}

const EO_MOD_SWIZZLE: u8x16 = u8x16::from_array([0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
const CO_MOD_SWIZZLE: u8x8 = u8x8::from_array([0, 1, 2, 0, 1, 2, 0, 0]);

static CUBE_3_SORTED_ORBIT_DEFS: LazyLock<Vec<OrbitDef>> = LazyLock::new(|| {
    vec![
        OrbitDef {
            piece_count: 8.try_into().unwrap(),
            orientation_count: 3.try_into().unwrap(),
        },
        OrbitDef {
            piece_count: 12.try_into().unwrap(),
            orientation_count: 2.try_into().unwrap(),
        },
    ]
});

impl PuzzleState for StackCube3Simd {
    type ReplaceComposeMeta = ();

    fn replace_compose(&mut self, a: &Self, b: &Self, _replace_compose_meta: &()) {
        // TODO: it is unclear for now if it will later be more efficient or
        // not to combine orientation/permutation into a single simd vector
        self.ep = a.ep.swizzle_dyn(b.ep);
        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(a.eo.swizzle_dyn(b.ep) + b.eo);
        // self.eo = (a.eo.swizzle_dyn(b.ep) + b.eo) % TWOS;
        self.cp = a.cp.swizzle_dyn(b.cp);
        self.co = CO_MOD_SWIZZLE.swizzle_dyn(a.co.swizzle_dyn(b.cp) + b.co);
        // self.co = (a.co.swizzle_dyn(b.cp) + b.co) % THREES;
    }

    fn expected_sorted_orbit_defs() -> Option<&'static [OrbitDef]> {
        Some(CUBE_3_SORTED_ORBIT_DEFS.as_slice())
    }

    fn from_sorted_transformations_unchecked(
        _sorted_orbit_defs: &[OrbitDef],
        sorted_transformations: &[Vec<(u8, u8)>],
    ) -> Result<Self, KSolveConversionError> {
        let corners_transformation = &sorted_transformations[0];
        let edges_transformation = &sorted_transformations[1];

        let mut ep = u8x16::splat(0);
        let mut eo = u8x16::splat(0);
        let mut cp = u8x8::splat(0);
        let mut co = u8x8::splat(0);

        for (i, &(perm, orientation_delta)) in edges_transformation.iter().enumerate() {
            ep[i] = perm;
            eo[i] = orientation_delta;
        }

        for (i, &(perm, orientation_delta)) in corners_transformation.iter().enumerate() {
            cp[i] = perm;
            co[i] = orientation_delta;
        }

        Ok(StackCube3Simd { ep, eo, cp, co })
    }
}

// pub struct StackEvenCubeSimd<const S_24S: usize> {
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32; S_24S],
// }

// pub struct HeapEvenCubeSimd {
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32],
// }

// pub struct StackOddCubeSimd<const S_24S: usize> {
//     ep: u8x16,
//     eo: u8x16,
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32; S_24S],
// }

// pub struct HeapOddCubeSimd {
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32],
// }

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;
    use puzzle_geometry::ksolve::KPUZZLE_3X3;
    use test::Bencher;

    static COMPOSE_R_F: [u8; 40] = [
        6, 1, 0, 4, 2, 5, 3, 7, 2, 2, 2, 1, 1, 0, 1, 0, 9, 3, 7, 2, 1, 5, 6, 0, 8, 4, 10, 11, 1, 1,
        0, 0, 1, 0, 0, 0, 0, 1, 0, 0,
    ];

    #[test]
    fn test_composition_stack() {
        let cube3_def: PuzzleDef<StackPuzzle<40>> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut solved = cube3_def.solved_state().unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        let f_move = cube3_def.find_move("F").unwrap();
        solved.replace_compose(
            &r_move.puzzle_state,
            &f_move.puzzle_state,
            &cube3_def.sorted_orbit_defs,
        );
        assert_eq!(solved.0, COMPOSE_R_F);
    }

    #[test]
    fn test_composition_heap() {
        let cube3_def: PuzzleDef<HeapPuzzle> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut solved = cube3_def.solved_state().unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        let f_move = cube3_def.find_move("F").unwrap();
        solved.replace_compose(
            &r_move.puzzle_state,
            &f_move.puzzle_state,
            &cube3_def.sorted_orbit_defs,
        );
        assert_eq!(solved.0.iter().as_slice(), COMPOSE_R_F);
    }

    #[test]
    fn test_composition_simd() {
        let cube3_def: PuzzleDef<StackCube3Simd> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut solved = cube3_def.solved_state().unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        let f_move = cube3_def.find_move("F").unwrap();
        solved.replace_compose(&r_move.puzzle_state, &f_move.puzzle_state, &());
        assert_eq!(solved.cp.as_array(), &COMPOSE_R_F[..8]);
        assert_eq!(solved.co.as_array(), &COMPOSE_R_F[8..16]);
        assert_eq!(&solved.ep.as_array()[..12], &COMPOSE_R_F[16..28]);
        assert_eq!(&solved.eo.as_array()[..12], &COMPOSE_R_F[28..40]);
    }

    #[bench]
    fn bench_compose_stack(b: &mut Bencher) {
        let cube3_def: PuzzleDef<StackPuzzle<40>> = (&*KPUZZLE_3X3).try_into().unwrap();
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
        let cube3_def: PuzzleDef<StackCube3Simd> = (&*KPUZZLE_3X3).try_into().unwrap();
        let mut solved = cube3_def.solved_state().unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        let f_move = cube3_def.find_move("F").unwrap();
        b.iter(|| {
            test::black_box(&mut solved).replace_compose(
                test::black_box(&r_move.puzzle_state),
                test::black_box(&f_move.puzzle_state),
                &(),
            );
        });
    }
}
