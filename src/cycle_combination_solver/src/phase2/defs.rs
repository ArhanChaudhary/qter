use super::puzzle::PuzzleDef;
use puzzle_geometry::{puzzles::Cube3PuzzleGeometry, PuzzleGeometryCore};
use qter_core::phase2_puzzle::{OrbitDef, PuzzleStateInterface};
use std::{marker::PhantomData, simd::Simd};

pub fn cube3_def<S>() -> PuzzleDef<S>
where
    S: PuzzleStateInterface,
{
    let cube3 = Cube3PuzzleGeometry::<S>(PhantomData);
    PuzzleDef {
        orbit_defs: cube3
            .pieces()
            .iter()
            .map(|&(size, orientation_mod)| OrbitDef {
                size: size as u8,
                orientation_mod,
            })
            .collect(),
        moves: cube3.moves(),
        name: "3x3x3".to_owned(),
    }
}

pub struct Cube3PuzzleSimd {
    pub ep: Simd<u8, 16>,
    eo: [u8; 12],
    pub cp: Simd<u8, 8>,
    co: [u8; 8],
}

impl PuzzleStateInterface for Cube3PuzzleSimd {
    fn solved(orbit_defs: &[OrbitDef]) -> Self {
        let ep = Simd::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let eo = [0; 12];
        let cp = Simd::from_array([0, 1, 2, 3, 4, 5, 6, 7]);
        let co = [0; 8];
        Cube3PuzzleSimd { ep, eo, cp, co }
    }

    fn from_orbit_states(slice: &[u8]) -> Self {
        let ep: [u8; 16] = slice[0..12]
            .iter()
            .chain([0, 0, 0, 0].iter())
            .copied()
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap();
        let ep: Simd<u8, 16> = ep.into();
        let eo = slice[12..24].try_into().unwrap();
        let cp = slice[24..32].try_into().unwrap();
        let co = slice[32..40].try_into().unwrap();
        Cube3PuzzleSimd { ep, eo, cp, co }
    }

    fn replace_compose(
        &mut self,
        move_a: &Cube3PuzzleSimd,
        move_b: &Cube3PuzzleSimd,
        orbit_defs: &[OrbitDef],
    ) {
        let new_ep = move_a.ep.swizzle_dyn(move_b.ep);
        let new_cp = move_a.cp.swizzle_dyn(move_b.cp);
        self.ep = new_ep;
        self.cp = new_cp;
    }
}
