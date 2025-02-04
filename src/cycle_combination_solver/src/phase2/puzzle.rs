use puzzle_geometry::KSolveMove;
use std::simd::{u8x16, u8x8};

use super::from_ksolve::{slice_try_from_ksolve, KSolveConversionError};

pub trait PuzzleState: for<'a> TryFrom<&'a KSolveMove> {
    type PuzzleMeta;

    fn solved(puzzle_meta: &Self::PuzzleMeta) -> Self;
    fn replace_compose(&mut self, move_a: &Self, move_b: &Self, puzzle_meta: &Self::PuzzleMeta);
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
    pub piece_count: u8,
    pub orientation_count: u8,
}

impl<P: PuzzleState> PuzzleDef<P> {
    pub fn find_move(&self, name: &str) -> Option<&Move<P>> {
        self.moves.iter().find(|def| def.name == name)
    }
}

impl<const N: usize> TryFrom<&KSolveMove> for StackPuzzle<N> {
    type Error = KSolveConversionError;

    fn try_from(ksolve_move: &KSolveMove) -> Result<Self, Self::Error> {
        let mut orbit_states = [0_u8; N];
        slice_try_from_ksolve(ksolve_move, &mut orbit_states)?;
        Ok(StackPuzzle(orbit_states))
    }
}

impl TryFrom<&KSolveMove> for HeapPuzzle {
    type Error = KSolveConversionError;

    fn try_from(ksolve_move: &KSolveMove) -> Result<Self, Self::Error> {
        let mut orbit_states = vec![
            0_u8;
            ksolve_move
                .zero_indexed_transformation()
                .iter()
                .map(|perm_and_ori| perm_and_ori.len() * 2)
                .sum()
        ]
        .into_boxed_slice();
        slice_try_from_ksolve(ksolve_move, &mut orbit_states)?;
        Ok(HeapPuzzle(orbit_states))
    }
}

impl<const N: usize> PuzzleState for StackPuzzle<N> {
    type PuzzleMeta = Vec<OrbitDef>;

    fn solved(orbit_defs: &Vec<OrbitDef>) -> Self {
        let mut orbit_states = [0_u8; N];
        slice_solved(orbit_defs, &mut orbit_states);
        StackPuzzle(orbit_states)
    }

    fn replace_compose(
        &mut self,
        move_a: &StackPuzzle<N>,
        move_b: &StackPuzzle<N>,
        puzzle_meta: &Vec<OrbitDef>,
    ) {
        slice_replace_compose(&mut self.0, &move_a.0, &move_b.0, puzzle_meta);
    }
}

impl PuzzleState for HeapPuzzle {
    type PuzzleMeta = Vec<OrbitDef>;

    fn solved(puzzle_meta: &Vec<OrbitDef>) -> Self {
        let mut orbit_states = vec![
            0_u8;
            puzzle_meta
                .iter()
                .map(|def| def.piece_count as usize * 2)
                .sum()
        ];
        slice_solved(puzzle_meta, &mut orbit_states);
        HeapPuzzle(orbit_states.into_boxed_slice())
    }

    fn replace_compose(
        &mut self,
        move_a: &HeapPuzzle,
        move_b: &HeapPuzzle,
        puzzle_meta: &Vec<OrbitDef>,
    ) {
        slice_replace_compose(&mut self.0, &move_a.0, &move_b.0, puzzle_meta);
    }
}

fn slice_solved(orbit_defs: &[OrbitDef], buf: &mut [u8]) {
    let mut base = 0;
    for &OrbitDef {
        piece_count: size, ..
    } in orbit_defs.iter()
    {
        for j in 1..size {
            buf[base as usize + j as usize] = j;
        }
        base += 2 * size;
    }
}

fn slice_replace_compose(orbit_states_mut: &mut [u8], a: &[u8], b: &[u8], orbit_defs: &[OrbitDef]) {
    debug_assert_eq!(
        orbit_defs
            .iter()
            .map(|v| (v.piece_count as usize) * 2)
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
        let piece_count = piece_count as usize;
        if orientation_count > 1 {
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
        } else {
            for i in 0..piece_count {
                let base_i = base + i;
                unsafe {
                    let pos = *a.get_unchecked(base + *b.get_unchecked(base_i) as usize);
                    *orbit_states_mut.get_unchecked_mut(base_i) = pos;
                    *orbit_states_mut.get_unchecked_mut(base_i + piece_count) = 0;
                }
            }
        }
        base += piece_count * 2;
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

    fn solved(_puzzle_meta: &()) -> Self {
        let ep = u8x16::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0, 0, 0, 0]);
        let eo = u8x16::splat(0);
        let cp = u8x8::from_array([0, 1, 2, 3, 4, 5, 6, 7]);
        let co = u8x8::splat(0);
        Cube3Simd { ep, eo, cp, co }
    }

    fn replace_compose(&mut self, move_a: &Cube3Simd, move_b: &Cube3Simd, _puzzle_meta: &()) {
        // TODO: it is unclear for now if it will later be more efficient or
        // not to combine orientation/permutation into a single simd vector
        self.ep = move_a.ep.swizzle_dyn(move_b.ep);
        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(move_a.eo.swizzle_dyn(move_b.ep) + move_b.eo);
        // self.eo = (move_a.eo.swizzle_dyn(move_b.ep) + move_b.eo) % TWOS;
        self.cp = move_a.cp.swizzle_dyn(move_b.cp);
        self.co = CO_MOD_SWIZZLE.swizzle_dyn(move_a.co.swizzle_dyn(move_b.cp) + move_b.co);
        // self.co = (move_a.co.swizzle_dyn(move_b.cp) + move_b.co) % THREES;
    }
}

impl TryFrom<&KSolveMove> for Cube3Simd {
    type Error = KSolveConversionError;

    fn try_from(ksolve_move: &KSolveMove) -> Result<Self, Self::Error> {
        let transformations = ksolve_move.zero_indexed_transformation();
        if transformations.len() != 2 {
            return Err(KSolveConversionError::InvalidSetCount(
                2,
                transformations.len(),
            ));
        }
        let (edges_transformation, corners_transformation) =
            match (transformations[0].len(), transformations[1].len()) {
                (12, 8) => (&transformations[0], &transformations[1]),
                (8, 12) => (&transformations[1], &transformations[0]),
                (12, _) => {
                    return Err(KSolveConversionError::InvalidPieceCount(
                        8,
                        transformations[1].len(),
                    ));
                }
                _ => {
                    return Err(KSolveConversionError::InvalidPieceCount(
                        12,
                        transformations[0].len(),
                    ));
                }
            };

        let mut ep = u8x16::splat(0);
        let mut eo = u8x16::splat(0);

        for (i, &(perm, orientation)) in edges_transformation.iter().enumerate() {
            ep[i] = perm
                .try_into()
                .map_err(|_| KSolveConversionError::PermutationOutOfRange(perm))?;
            eo[i] = orientation;
        }

        let mut cp = u8x8::splat(0);
        let mut co = u8x8::splat(0);

        for (i, &(perm, orientation)) in corners_transformation.iter().enumerate() {
            cp[i] = perm
                .try_into()
                .map_err(|_| KSolveConversionError::PermutationOutOfRange(perm))?;
            co[i] = orientation;
        }

        Ok(Cube3Simd { ep, eo, cp, co })
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;
    use puzzle_geometry::puzzles::KPUZZLE_3X3;
    use test::Bencher;

    static COMPOSE_R_F: [u8; 40] = [
        9, 3, 7, 2, 1, 5, 6, 0, 8, 4, 10, 11, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 6, 1, 0, 4, 2, 5,
        3, 7, 2, 2, 2, 1, 1, 0, 1, 0,
    ];

    #[test]
    fn test_composition_stack() {
        let cube3_def = PuzzleDef::<StackPuzzle<40>>::try_from(&*KPUZZLE_3X3).unwrap();
        let mut solved = StackPuzzle::<40>::solved(&cube3_def.orbit_defs);
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
        let mut solved = HeapPuzzle::solved(&cube3_def.orbit_defs);
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
        let mut solved = Cube3Simd::solved(&());
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
        let mut solved = Cube3Simd::solved(&());
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
