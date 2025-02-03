use puzzle_geometry::PuzzleGeometryCore;
use qter_core::phase2_puzzle::{Move, OrbitDef, PuzzleState};
use std::simd::{u8x16, u8x8, Simd};

pub struct PuzzleDef<P: PuzzleState> {
    pub name: String,
    pub orbit_defs: Vec<OrbitDef>,
    pub moves: Vec<Move<P>>,
}

impl<P: PuzzleState> PuzzleDef<P> {
    pub fn from_puzzle_geometry(puzzle_geometry: impl PuzzleGeometryCore<P>) -> Self {
        PuzzleDef {
            name: "3x3x3".to_owned(),
            orbit_defs: puzzle_geometry
                .pieces()
                .iter()
                .map(|&(size, orientation_mod)| OrbitDef {
                    size: size as u8,
                    orientation_mod,
                })
                .collect(),
            moves: puzzle_geometry.moves(),
        }
    }

    pub fn get_move(&self, name: &str) -> Option<&Move<P>> {
        self.moves.iter().find(|def| def.name == name)
    }
}

pub struct StackPuzzle<const N: usize>([u8; N]);
pub struct HeapPuzzle(Box<[u8]>);

impl<const N: usize> PuzzleState for StackPuzzle<N> {
    type PuzzleMeta = Vec<OrbitDef>;

    fn solved(orbit_defs: &Vec<OrbitDef>) -> Self {
        let mut orbit_states = [0_u8; N];
        let mut base = 0;
        for &OrbitDef { size, .. } in orbit_defs.iter() {
            for j in 1..size {
                orbit_states[base as usize + j as usize] = j;
            }
            base += 2 * size;
        }
        StackPuzzle(orbit_states)
    }

    fn from_orbit_states(slice: &[u8]) -> Self {
        StackPuzzle(slice.try_into().unwrap())
    }

    fn replace_compose(
        &mut self,
        move_a: &StackPuzzle<N>,
        move_b: &StackPuzzle<N>,
        orbit_defs: &Vec<OrbitDef>,
    ) {
        slice_replace_compose(&mut self.0, &move_a.0, &move_b.0, orbit_defs);
    }
}

impl PuzzleState for HeapPuzzle {
    type PuzzleMeta = Vec<OrbitDef>;

    fn solved(puzzle_meta: &Vec<OrbitDef>) -> Self {
        let mut orbit_states =
            vec![0_u8; puzzle_meta.iter().map(|def| def.size as usize * 2).sum()];
        let mut base = 0;
        for &OrbitDef { size, .. } in puzzle_meta.iter() {
            for j in 1..size {
                orbit_states[base as usize + j as usize] = j;
            }
            base += 2 * size;
        }
        HeapPuzzle(orbit_states.into_boxed_slice())
    }

    fn from_orbit_states(slice: &[u8]) -> Self {
        HeapPuzzle(slice.into())
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

fn slice_replace_compose(
    orbit_states_mut: &mut [u8],
    a: &[u8],
    b: &[u8],
    puzzle_meta: &[OrbitDef],
) {
    debug_assert_eq!(
        puzzle_meta
            .iter()
            .map(|v| (v.size as usize) * 2)
            .sum::<usize>(),
        orbit_states_mut.len()
    );
    debug_assert_eq!(orbit_states_mut.len(), a.len());
    debug_assert_eq!(a.len(), b.len());

    let mut base = 0;
    for &OrbitDef {
        size,
        orientation_mod,
    } in puzzle_meta
    {
        let size = size as usize;
        if orientation_mod > 1 {
            for i in 0..size {
                let base_i = base + i;
                unsafe {
                    let pos = a.get_unchecked(base + *b.get_unchecked(base_i) as usize);
                    let a_ori = a.get_unchecked(base + *b.get_unchecked(base_i) as usize + size);
                    let b_ori = b.get_unchecked(base_i + size);
                    *orbit_states_mut.get_unchecked_mut(base_i) = *pos;
                    *orbit_states_mut.get_unchecked_mut(base_i + size) =
                        (*a_ori + *b_ori) % orientation_mod;
                }
            }
        } else {
            for i in 0..size {
                let base_i = base + i;
                unsafe {
                    let pos = *a.get_unchecked(base + *b.get_unchecked(base_i) as usize);
                    *orbit_states_mut.get_unchecked_mut(base_i) = pos;
                    *orbit_states_mut.get_unchecked_mut(base_i + size) = 0;
                }
            }
        }
        base += size * 2;
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

const EO_MOD_SWIZZLE: u8x16 = Simd::from_array([0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
const CO_MOD_SWIZZLE: u8x8 = Simd::from_array([0, 1, 2, 0, 1, 2, 0, 0]);

impl PuzzleState for Cube3Simd {
    type PuzzleMeta = ();

    fn solved(_puzzle_meta: &()) -> Self {
        let ep = u8x16::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0, 0, 0, 0]);
        let eo = u8x16::splat(0);
        let cp = u8x8::from_array([0, 1, 2, 3, 4, 5, 6, 7]);
        let co = u8x8::splat(0);
        Cube3Simd { ep, eo, cp, co }
    }

    fn from_orbit_states(slice: &[u8]) -> Self {
        let mut ep = u8x16::splat(0);
        ep.as_mut_array()[..12].copy_from_slice(&slice[..12]);
        let mut eo = u8x16::splat(0);
        eo.as_mut_array()[..12].copy_from_slice(&slice[12..24]);
        let cp = slice[24..32].try_into().unwrap();
        let co = slice[32..40].try_into().unwrap();
        Cube3Simd { ep, eo, cp, co }
    }

    fn replace_compose(&mut self, move_a: &Cube3Simd, move_b: &Cube3Simd, _puzzle_meta: &()) {
        // FIXME: it is unclear for now if it will later be more efficient or
        // not to combine orientation/permutation into a single simd vector
        self.ep = move_a.ep.swizzle_dyn(move_b.ep);
        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(move_a.eo.swizzle_dyn(move_b.ep) + move_b.eo);
        // self.eo = (move_a.eo.swizzle_dyn(move_b.ep) + move_b.eo) % TWOS;
        self.cp = move_a.cp.swizzle_dyn(move_b.cp);
        self.co = CO_MOD_SWIZZLE.swizzle_dyn(move_a.co.swizzle_dyn(move_b.cp) + move_b.co);
        // self.co = (move_a.co.swizzle_dyn(move_b.cp) + move_b.co) % THREES;
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;
    use puzzle_geometry::puzzles::Cube3PuzzleGeometry;
    use std::marker::PhantomData;
    use test::Bencher;

    static COMPOSE_R_F: [u8; 40] = [
        9, 3, 7, 2, 1, 5, 6, 0, 8, 4, 10, 11, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 6, 1, 0, 4, 2, 5,
        3, 7, 2, 2, 2, 1, 1, 0, 1, 0,
    ];

    #[test]
    fn test_composition_stack() {
        let cube3_def = PuzzleDef::from_puzzle_geometry(Cube3PuzzleGeometry(PhantomData));
        let mut solved = StackPuzzle::<40>::solved(&cube3_def.orbit_defs);
        let r_move = cube3_def.get_move("R").unwrap();
        let f_move = cube3_def.get_move("F").unwrap();
        solved.replace_compose(&r_move.delta, &f_move.delta, &cube3_def.orbit_defs);
        assert_eq!(solved.0, COMPOSE_R_F);
    }

    #[test]
    fn test_composition_heap() {
        let cube3_def = PuzzleDef::from_puzzle_geometry(Cube3PuzzleGeometry(PhantomData));
        let mut solved = HeapPuzzle::solved(&cube3_def.orbit_defs);
        let r_move = cube3_def.get_move("R").unwrap();
        let f_move = cube3_def.get_move("F").unwrap();
        solved.replace_compose(&r_move.delta, &f_move.delta, &cube3_def.orbit_defs);
        assert_eq!(solved.0.iter().as_slice(), COMPOSE_R_F);
    }

    #[test]
    fn test_composition_simd() {
        let cube3_def = PuzzleDef::from_puzzle_geometry(Cube3PuzzleGeometry(PhantomData));
        let mut solved = Cube3Simd::solved(&());
        let r_move = cube3_def.get_move("R").unwrap();
        let f_move = cube3_def.get_move("F").unwrap();
        solved.replace_compose(&r_move.delta, &f_move.delta, &());
        assert_eq!(&solved.ep.as_array()[..12], &COMPOSE_R_F[..12]);
        assert_eq!(&solved.eo.as_array()[..12], &COMPOSE_R_F[12..24]);
        assert_eq!(solved.cp.as_array(), &COMPOSE_R_F[24..32]);
        assert_eq!(solved.co.as_array(), &COMPOSE_R_F[32..40]);
    }

    #[bench]
    fn bench_compose(b: &mut Bencher) {
        let cube3_def = PuzzleDef::from_puzzle_geometry(Cube3PuzzleGeometry(PhantomData));
        let mut solved = Cube3Simd::solved(&());
        let r_move = cube3_def.get_move("R").unwrap();
        let f_move = cube3_def.get_move("F").unwrap();
        b.iter(|| {
            test::black_box(&mut solved).replace_compose(
                test::black_box(&r_move.delta),
                test::black_box(&f_move.delta),
                &(),
            );
        });
    }
}
