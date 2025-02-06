use puzzle_geometry::ksolve::{KSolve, KSolveMove};
use std::{
    num::NonZeroU8,
    simd::{u8x16, u8x8},
};
use thiserror::Error;
pub trait PuzzleState
where
    Self: Sized,
{
    type PuzzleMeta;

    /// Get the solved state of the puzzle
    // TODO: this should be a method from KSolve as that will provide information
    // about identical pieces. This works for now but will definitely be changed
    // in the future.
    fn solved(puzzle_meta: &Self::PuzzleMeta) -> Result<Self, KSolveConversionError>;
    /// Compose two puzzle states in place
    fn replace_compose(&mut self, a: &Self, b: &Self, puzzle_meta: &Self::PuzzleMeta);
    /// Create a puzzle state from a ksolve move without checking if the move is
    /// part of KSolve
    fn from_ksolve_meta_unchecked(
        ksolve: &KSolve,
        ksolve_move: &KSolveMove,
    ) -> Result<Self, KSolveConversionError>;
    /// Create a puzzle state from a ksolve move, checking if move is part of
    /// KSolve
    fn from_ksolve_meta(
        ksolve: &KSolve,
        ksolve_move: &KSolveMove,
    ) -> Result<Self, KSolveConversionError> {
        if !ksolve
            .moves()
            .iter()
            .any(|ksolve_move_iter| std::ptr::eq(ksolve_move_iter, ksolve_move))
        {
            Err(KSolveConversionError::InvalidKSolveMeta(
                "KSolveMove not in KSolve".to_owned(),
            ))
        } else {
            Self::from_ksolve_meta_unchecked(ksolve, ksolve_move)
        }
    }
}

pub struct StackPuzzle<const N: usize>([u8; N]);
pub struct HeapPuzzle(Box<[u8]>);

pub struct PuzzleDef<P: PuzzleState> {
    pub moves: Vec<Move<P>>,
    pub orbit_defs: Vec<OrbitDef>,
    pub name: String,
}

#[repr(C)]
pub struct Move<P: PuzzleState> {
    pub transformation: P,
    pub name: String,
}

pub struct OrbitDef {
    pub piece_count: NonZeroU8,
    pub orientation_count: NonZeroU8,
}

#[derive(Error, Debug)]
pub enum KSolveConversionError {
    #[error("Phase 2 does not currently support puzzles with set sizes larger than 255, but it will in the future")]
    SetSizeTooBig,
    #[error("Invalid ksolve meta: {0}")]
    InvalidKSolveMeta(String),
    #[error("Not enough buffer space to convert move")]
    NotEnoughBufferSpace,
}

impl<const N: usize> PuzzleState for StackPuzzle<N> {
    type PuzzleMeta = Vec<OrbitDef>;

    fn solved(orbit_defs: &Vec<OrbitDef>) -> Result<Self, KSolveConversionError> {
        let mut orbit_states = [0_u8; N];
        slice_solved(orbit_defs, &mut orbit_states)?;
        Ok(StackPuzzle(orbit_states))
    }

    fn replace_compose(
        &mut self,
        a: &StackPuzzle<N>,
        b: &StackPuzzle<N>,
        puzzle_meta: &Vec<OrbitDef>,
    ) {
        slice_replace_compose(&mut self.0, &a.0, &b.0, puzzle_meta);
    }

    fn from_ksolve_meta_unchecked(
        ksolve: &KSolve,
        ksolve_move: &KSolveMove,
    ) -> Result<Self, KSolveConversionError> {
        let mut orbit_states = [0_u8; N];
        ksolve_meta_to_slice_unchecked(ksolve, ksolve_move, &mut orbit_states)?;
        Ok(StackPuzzle(orbit_states))
    }
}

impl PuzzleState for HeapPuzzle {
    type PuzzleMeta = Vec<OrbitDef>;

    fn solved(orbit_defs: &Vec<OrbitDef>) -> Result<Self, KSolveConversionError> {
        let mut orbit_states = vec![
            0_u8;
            orbit_defs
                .iter()
                .map(|def| def.piece_count.get() as usize * 2)
                .sum()
        ];
        slice_solved(orbit_defs, &mut orbit_states)?;
        Ok(HeapPuzzle(orbit_states.into_boxed_slice()))
    }

    fn replace_compose(&mut self, a: &HeapPuzzle, b: &HeapPuzzle, puzzle_meta: &Vec<OrbitDef>) {
        slice_replace_compose(&mut self.0, &a.0, &b.0, puzzle_meta);
    }

    fn from_ksolve_meta_unchecked(
        ksolve: &KSolve,
        ksolve_move: &KSolveMove,
    ) -> Result<Self, KSolveConversionError> {
        let mut orbit_states = vec![
            0_u8;
            ksolve_move
                .zero_indexed_transformation()
                .iter()
                .map(|perm_and_ori| perm_and_ori.len() * 2)
                .sum()
        ]
        .into_boxed_slice();
        ksolve_meta_to_slice_unchecked(ksolve, ksolve_move, &mut orbit_states)?;
        Ok(HeapPuzzle(orbit_states))
    }
}

fn slice_solved(orbit_defs: &[OrbitDef], buf: &mut [u8]) -> Result<(), KSolveConversionError> {
    let mut base = 0;
    for &OrbitDef { piece_count, .. } in orbit_defs.iter() {
        for j in 1..piece_count.get() {
            *buf.get_mut(base as usize + j as usize)
                .ok_or(KSolveConversionError::NotEnoughBufferSpace)? = j;
        }
        base += 2 * piece_count.get();
    }
    Ok(())
}

fn slice_replace_compose(orbit_states_mut: &mut [u8], a: &[u8], b: &[u8], orbit_defs: &[OrbitDef]) {
    debug_assert_eq!(
        orbit_defs
            .iter()
            .map(|v| (v.piece_count.get() as usize) * 2)
            .sum::<usize>(),
        orbit_states_mut.len()
    );
    debug_assert_eq!(orbit_states_mut.len(), a.len());
    debug_assert_eq!(a.len(), b.len());

    let mut base = 0;
    for &OrbitDef {
        piece_count,
        orientation_count,
    } in orbit_defs
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

fn ksolve_meta_to_slice_unchecked(
    ksolve: &KSolve,
    ksolve_move: &KSolveMove,
    buf: &mut [u8],
) -> Result<(), KSolveConversionError> {
    let mut i = 0;
    for (transformation, ksolve_set) in ksolve_move
        .zero_indexed_transformation()
        .iter()
        .zip(ksolve.sets())
    {
        let piece_count = ksolve_set.piece_count().get() as usize;
        for j in 0..piece_count {
            let (perm, orientation_delta) = transformation[j];
            *buf.get_mut(i + j + piece_count)
                .ok_or(KSolveConversionError::NotEnoughBufferSpace)? = orientation_delta;
            buf[i + j] = perm.try_into().unwrap();
        }
        i += piece_count * 2;
    }
    Ok(())
}

impl<P: PuzzleState> PuzzleDef<P> {
    pub fn find_move(&self, name: &str) -> Option<&Move<P>> {
        self.moves.iter().find(|def| def.name == name)
    }
}

impl<P: PuzzleState> TryFrom<&KSolve> for PuzzleDef<P> {
    type Error = KSolveConversionError;

    fn try_from(ksolve: &KSolve) -> Result<Self, Self::Error> {
        let orbit_defs: Vec<OrbitDef> = ksolve
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

        let moves: Vec<Move<P>> = ksolve
            .moves()
            .iter()
            .map(|ksolve_move| {
                Ok(Move {
                    name: ksolve_move.name().to_owned(),
                    transformation: P::from_ksolve_meta_unchecked(ksolve, ksolve_move)?,
                })
            })
            .collect::<Result<_, KSolveConversionError>>()?;

        Ok(PuzzleDef {
            moves,
            orbit_defs,
            name: ksolve.name().to_owned(),
        })
    }
}

// TODO: Utilize #[cfg(simd8)] #[cfg(simd16)] and #[cfg(simd32)] for differing
// implementations
pub struct Cube3Simd {
    pub ep: u8x16,
    pub eo: u8x16,
    pub cp: u8x8,
    pub co: u8x8,
}

const EO_MOD_SWIZZLE: u8x16 = u8x16::from_array([0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
const CO_MOD_SWIZZLE: u8x8 = u8x8::from_array([0, 1, 2, 0, 1, 2, 0, 0]);

impl PuzzleState for Cube3Simd {
    type PuzzleMeta = ();

    fn solved(_puzzle_meta: &()) -> Result<Self, KSolveConversionError> {
        let ep = u8x16::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0, 0, 0, 0]);
        let eo = u8x16::splat(0);
        let cp = u8x8::from_array([0, 1, 2, 3, 4, 5, 6, 7]);
        let co = u8x8::splat(0);
        Ok(Cube3Simd { ep, eo, cp, co })
    }

    fn replace_compose(&mut self, a: &Cube3Simd, b: &Cube3Simd, _puzzle_meta: &()) {
        // TODO: it is unclear for now if it will later be more efficient or
        // not to combine orientation/permutation into a single simd vector
        self.ep = a.ep.swizzle_dyn(b.ep);
        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(a.eo.swizzle_dyn(b.ep) + b.eo);
        // self.eo = (a.eo.swizzle_dyn(b.ep) + b.eo) % TWOS;
        self.cp = a.cp.swizzle_dyn(b.cp);
        self.co = CO_MOD_SWIZZLE.swizzle_dyn(a.co.swizzle_dyn(b.cp) + b.co);
        // self.co = (a.co.swizzle_dyn(b.cp) + b.co) % THREES;
    }

    fn from_ksolve_meta_unchecked(
        ksolve: &KSolve,
        ksolve_move: &KSolveMove,
    ) -> Result<Self, KSolveConversionError> {
        if ksolve.sets().len() != 2 {
            return Err(KSolveConversionError::InvalidKSolveMeta(format!(
                "Expected 2 sets in KSolve, found {}",
                ksolve.sets().len()
            )));
        }
        // TODO: how should a set ordering be enforced for identical sets? This
        // will be an issue in the future because the fields for identical sets
        // will be named in the struct. For now, we will just check the set
        // name, this is done over here in the 3x3x3 despite sets not being
        // identical to maintain consistency. This will probably fail a few test
        // cases in the future.
        let maybe_edges_set = &ksolve.sets()[0];
        match (
            maybe_edges_set.name(),
            maybe_edges_set.piece_count().get(),
            maybe_edges_set.orientation_count().get(),
        ) {
            ("Edges", 12, 2) => {}
            _ => {
                return Err(KSolveConversionError::InvalidKSolveMeta(
                    "First set must be a valid set for edges".to_owned(),
                ))
            }
        }

        let maybe_corners_set = &ksolve.sets()[1];
        match (
            maybe_corners_set.name(),
            maybe_corners_set.piece_count().get(),
            maybe_corners_set.orientation_count().get(),
        ) {
            ("Corners", 8, 3) => {}
            _ => {
                return Err(KSolveConversionError::InvalidKSolveMeta(
                    "Second set must be a valid set for corners".to_owned(),
                ))
            }
        }

        let transformations = ksolve_move.zero_indexed_transformation();
        let edges_transformation = &transformations[0];
        let corners_transformation = &transformations[1];

        let mut ep = u8x16::splat(0);
        let mut eo = u8x16::splat(0);

        for (i, &(perm, orientation_delta)) in edges_transformation.iter().enumerate() {
            ep[i] = perm.try_into().unwrap();
            eo[i] = orientation_delta;
        }

        let mut cp = u8x8::splat(0);
        let mut co = u8x8::splat(0);

        for (i, &(perm, orientation_delta)) in corners_transformation.iter().enumerate() {
            cp[i] = perm.try_into().unwrap();
            co[i] = orientation_delta;
        }

        Ok(Cube3Simd { ep, eo, cp, co })
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;
    use puzzle_geometry::ksolve::KPUZZLE_3X3;
    use test::Bencher;

    static COMPOSE_R_F: [u8; 40] = [
        9, 3, 7, 2, 1, 5, 6, 0, 8, 4, 10, 11, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 6, 1, 0, 4, 2, 5,
        3, 7, 2, 2, 2, 1, 1, 0, 1, 0,
    ];

    #[test]
    fn test_composition_stack() {
        let cube3_def = PuzzleDef::<StackPuzzle<40>>::try_from(&*KPUZZLE_3X3).unwrap();
        let mut solved = StackPuzzle::<40>::solved(&cube3_def.orbit_defs).unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        let f_move = cube3_def.find_move("F").unwrap();
        solved.replace_compose(
            &r_move.transformation,
            &f_move.transformation,
            &cube3_def.orbit_defs,
        );
        assert_eq!(solved.0, COMPOSE_R_F);
    }

    #[test]
    fn test_composition_heap() {
        let cube3_def = PuzzleDef::<HeapPuzzle>::try_from(&*KPUZZLE_3X3).unwrap();
        let mut solved = HeapPuzzle::solved(&cube3_def.orbit_defs).unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        let f_move = cube3_def.find_move("F").unwrap();
        solved.replace_compose(
            &r_move.transformation,
            &f_move.transformation,
            &cube3_def.orbit_defs,
        );
        assert_eq!(solved.0.iter().as_slice(), COMPOSE_R_F);
    }

    #[test]
    fn test_composition_simd() {
        let cube3_def = PuzzleDef::<Cube3Simd>::try_from(&*KPUZZLE_3X3).unwrap();
        let mut solved = Cube3Simd::solved(&()).unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        let f_move = cube3_def.find_move("F").unwrap();
        solved.replace_compose(&r_move.transformation, &f_move.transformation, &());
        assert_eq!(&solved.ep.as_array()[..12], &COMPOSE_R_F[..12]);
        assert_eq!(&solved.eo.as_array()[..12], &COMPOSE_R_F[12..24]);
        assert_eq!(solved.cp.as_array(), &COMPOSE_R_F[24..32]);
        assert_eq!(solved.co.as_array(), &COMPOSE_R_F[32..40]);
    }

    #[bench]
    fn bench_compose(b: &mut Bencher) {
        let cube3_def = PuzzleDef::<Cube3Simd>::try_from(&*KPUZZLE_3X3).unwrap();
        let mut solved = Cube3Simd::solved(&()).unwrap();
        let r_move = cube3_def.find_move("R").unwrap();
        let f_move = cube3_def.find_move("F").unwrap();
        b.iter(|| {
            test::black_box(&mut solved).replace_compose(
                test::black_box(&r_move.transformation),
                test::black_box(&f_move.transformation),
                &(),
            );
        });
    }
}
