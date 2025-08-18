//! SIMD optimized implementations for 3x3 orbits during pruning table
//! generation.

use crate::{
    orbit_puzzle::{OrbitPuzzleState, OrbitPuzzleStateExtra, OrbitPuzzleStateImplementor},
    puzzle::{AuxMemRefMut, OrbitDef},
};
use std::{hash::Hash, num::NonZeroU8};

pub mod avx2;
pub mod simd8and16;

#[derive(PartialEq, Clone)]
pub struct Cube3Edges;

#[derive(PartialEq, Clone)]
pub struct Cube3Corners;

#[allow(unused)]
impl OrbitPuzzleState for Cube3Corners {
    unsafe fn replace_compose(
        &mut self,
        a: &OrbitPuzzleStateImplementor,
        b: &OrbitPuzzleStateImplementor,
        orbit_def: OrbitDef,
    ) {
        todo!()
    }

    unsafe fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type_orbit: &[(NonZeroU8, bool)],
        orbit_def: OrbitDef,
        aux_mem: AuxMemRefMut,
    ) -> bool {
        todo!()
    }

    unsafe fn exact_hasher(&self, orbit_def: OrbitDef) -> u64 {
        todo!()
    }
}

#[allow(unused)]
impl OrbitPuzzleStateExtra for Cube3Corners {
    fn approximate_hash(&self) -> impl Hash {
        todo!()
    }

    unsafe fn from_orbit_transformation_unchecked<B: AsRef<[u8]>>(
        perm: B,
        ori: B,
        orbit_def: OrbitDef,
    ) -> Self {
        todo!()
    }
}

#[allow(unused)]
impl OrbitPuzzleState for Cube3Edges {
    unsafe fn replace_compose(
        &mut self,
        a: &OrbitPuzzleStateImplementor,
        b: &OrbitPuzzleStateImplementor,
        orbit_def: OrbitDef,
    ) {
        todo!()
    }

    unsafe fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type_orbit: &[(NonZeroU8, bool)],
        orbit_def: OrbitDef,
        aux_mem: AuxMemRefMut,
    ) -> bool {
        todo!()
    }

    unsafe fn exact_hasher(&self, orbit_def: OrbitDef) -> u64 {
        todo!()
    }
}

#[allow(unused)]
impl OrbitPuzzleStateExtra for Cube3Edges {
    fn approximate_hash(&self) -> impl Hash {
        todo!()
    }

    unsafe fn from_orbit_transformation_unchecked<B: AsRef<[u8]>>(
        perm: B,
        ori: B,
        orbit_def: OrbitDef,
    ) -> Self {
        todo!()
    }
}
