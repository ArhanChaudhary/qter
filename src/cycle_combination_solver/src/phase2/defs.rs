use super::puzzle::PuzzleDef;
use puzzle_geometry::{puzzles::Cube3PuzzleGeometry, PuzzleGeometryCore};
use qter_core::phase2_puzzle::{OrbitDef, PuzzleState, PuzzleStateInterface, PuzzleStorage};
use std::{marker::PhantomData, simd::Simd};

pub fn cube3_def<S>() -> PuzzleDef<S>
where
    S: PuzzleStorage,
    PuzzleState<S>: PuzzleStateInterface<S>,
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

struct Cube3PuzzleSimd {
    ep: Simd<u8, 16>,
    cp: Simd<u8, 8>,
}

impl PuzzleStorage for Cube3PuzzleSimd {
    type Buf = Self;
}

impl PuzzleStateInterface<Cube3PuzzleSimd> for PuzzleState<Cube3PuzzleSimd> {
    fn solved(orbit_defs: &[OrbitDef]) -> Self {
        todo!()
    }

    fn from_orbit_states(slice: &[u8]) -> Self {
        todo!()
    }

    fn replace_compose(
        &mut self,
        move_a: &qter_core::phase2_puzzle::Move<Cube3PuzzleSimd>,
        move_b: &qter_core::phase2_puzzle::Move<Cube3PuzzleSimd>,
        orbit_defs: &[OrbitDef],
    ) {
        todo!()
    }
}
