#![cfg_attr(not(simd32), allow(dead_code, unused_variables))]

use super::common::CUBE_3_SORTED_ORBIT_DEFS;
use crate::phase2::puzzle::{KSolveConversionError, OrbitDef, OrientedPartition, PuzzleState};
use std::{
    hash::{Hash, Hasher},
    simd::u8x32,
};

#[derive(Clone, Debug)]
pub struct Cube3 {
    perm: u8x32,
    ori: u8x32,
}

impl PartialEq for Cube3 {
    fn eq(&self, other: &Self) -> bool {
        todo!();
    }
}

impl Hash for Cube3 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        todo!();
    }
}

impl PuzzleState for Cube3 {
    type MultiBv = [u16; 2];

    fn new_multi_bv(_sorted_orbit_defs: &[OrbitDef]) -> [u16; 2] {
        Default::default()
    }

    fn validate_sorted_orbit_defs(
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<(), KSolveConversionError> {
        if sorted_orbit_defs == CUBE_3_SORTED_ORBIT_DEFS.as_slice() {
            Ok(())
        } else {
            Err(KSolveConversionError::InvalidOrbitDefs(
                CUBE_3_SORTED_ORBIT_DEFS.to_vec(),
                sorted_orbit_defs.to_vec(),
            ))
        }
    }

    fn from_sorted_transformations_unchecked(
        sorted_transformations: &[Vec<(u8, u8)>],
        _sorted_orbit_defs: &[OrbitDef],
    ) -> Self {
        todo!();
    }

    fn replace_compose(&mut self, a: &Self, b: &Self, _sorted_orbit_defs: &[OrbitDef]) {
        todo!();
    }

    fn replace_inverse(&mut self, a: &Self, _sorted_orbit_defs: &[OrbitDef]) {
        todo!();
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        multi_bv: [u16; 2],
        _sorted_orbit_defs: &[OrbitDef],
    ) -> bool {
        todo!();
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
