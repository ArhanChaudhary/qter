//! SIMD optimized implementations for 3x3 orbits during pruning table
//! generation.

use crate::{
    orbit_puzzle::{OrbitPuzzleConstructor, OrbitPuzzleState, OrbitPuzzleStateImplementor},
    puzzle::{AuxMemRefMut, BrandedOrbitDef},
};
use std::{hash::Hash, num::NonZeroU8};

pub mod avx2;
pub mod simd8and16;

#[derive(PartialEq, Clone)]
pub struct Cube3Edges;

#[derive(PartialEq, Clone)]
pub struct Cube3Corners;

#[allow(unused)]
impl<'id2> OrbitPuzzleState<'id2> for Cube3Corners {
    unsafe fn replace_compose(
        &mut self,
        a: &OrbitPuzzleStateImplementor<'id2>,
        b: &OrbitPuzzleStateImplementor<'id2>,
        branded_orbit_def: BrandedOrbitDef<'id2>,
    ) {
        todo!()
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type_orbit: &[(NonZeroU8, bool)],
        branded_orbit_def: BrandedOrbitDef<'id2>,
        aux_mem: AuxMemRefMut<'id2, '_>,
    ) -> bool {
        todo!()
    }

    fn exact_hasher(&self, branded_orbit_def: BrandedOrbitDef<'id2>) -> u64 {
        todo!()
    }
}

#[allow(unused)]
impl<'id2> OrbitPuzzleConstructor<'id2> for Cube3Corners {
    fn from_orbit_transformation_unchecked<B: AsRef<[u8]>>(
        perm: B,
        ori: B,
        branded_orbit_def: BrandedOrbitDef<'id2>,
    ) -> Self {
        todo!()
    }

    fn approximate_hash(&self) -> impl Hash {
        todo!()
    }
}

#[allow(unused)]
impl<'id2> OrbitPuzzleState<'id2> for Cube3Edges {
    unsafe fn replace_compose(
        &mut self,
        a: &OrbitPuzzleStateImplementor<'id2>,
        b: &OrbitPuzzleStateImplementor<'id2>,
        branded_orbit_def: BrandedOrbitDef<'id2>,
    ) {
        todo!()
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type_orbit: &[(NonZeroU8, bool)],
        branded_orbit_def: BrandedOrbitDef<'id2>,
        aux_mem: AuxMemRefMut<'id2, '_>,
    ) -> bool {
        todo!()
    }

    fn exact_hasher(&self, branded_orbit_def: BrandedOrbitDef<'id2>) -> u64 {
        todo!()
    }
}

#[allow(unused)]
impl<'id2> OrbitPuzzleConstructor<'id2> for Cube3Edges {
    fn from_orbit_transformation_unchecked<B: AsRef<[u8]>>(
        perm: B,
        ori: B,
        branded_orbit_def: BrandedOrbitDef<'id2>,
    ) -> Self {
        todo!()
    }

    fn approximate_hash(&self) -> impl Hash {
        todo!()
    }
}
