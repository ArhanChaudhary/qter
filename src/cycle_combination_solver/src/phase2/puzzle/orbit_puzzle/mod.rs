use super::{KSolveConversionError, MultiBvInterface, OrbitDef, OrientedPartition, PuzzleState};
use std::num::NonZeroU8;

pub trait OrbitPuzzleState {
    type MultiBv: MultiBvInterface;

    fn new_multi_bv(sorted_orbit_cycle_type: &[(NonZeroU8, bool)]) -> Self::MultiBv;
    fn from_orbit_transformation_unchecked(perm: &[u8], ori: &[u8], orbit_def: OrbitDef) -> Self;
    fn replace_compose(&mut self, a: &Self, b: &Self, orbit_def: OrbitDef);
    fn induces_sorted_orbit_cycle_type(
        &self,
        sorted_orbit_cycle_type: &[(NonZeroU8, bool)],
        orbit_def: OrbitDef,
        multi_bv: <Self::MultiBv as MultiBvInterface>::MultiBvReusableRef<'_>,
    ) -> bool;
    fn exact_hash(&self, orbit_def: OrbitDef) -> u64;
}

#[derive(Clone, PartialEq, Debug)]
pub struct SliceOrbitPuzzle(Box<[u8]>);

impl PuzzleState for SliceOrbitPuzzle {
    type MultiBv = Box<[u8]>;

    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        todo!()
    }

    fn try_from_transformation_meta(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError> {
        unimplemented!()
    }

    fn replace_compose(&mut self, a: &Self, b: &Self, sorted_orbit_defs: &[OrbitDef]) {
        todo!()
    }

    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: &[OrbitDef]) {
        todo!()
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: <Self::MultiBv as MultiBvInterface>::MultiBvReusableRef<'_>,
    ) -> bool {
        todo!()
    }

    fn next_orbit_identifer(orbit_identifier: usize, orbit_def: OrbitDef) -> usize {
        todo!()
    }

    fn orbit_bytes(&self, orbit_identifier: usize, orbit_def: OrbitDef) -> (&[u8], &[u8]) {
        todo!()
    }

    fn exact_hash_orbit(&self, orbit_identifier: usize, orbit_def: OrbitDef) -> u64 {
        todo!()
    }

    fn approximate_hash_orbit(
        &self,
        orbit_identifier: usize,
        orbit_def: OrbitDef,
    ) -> impl std::hash::Hash {
        todo!()
    }
}

impl OrbitPuzzleState for SliceOrbitPuzzle {
    type MultiBv = Box<[u8]>;

    fn new_multi_bv(sorted_orbit_cycle_type: &[(NonZeroU8, bool)]) -> Self::MultiBv {
        todo!()
    }

    fn replace_compose(&mut self, a: &Self, b: &Self, orbit_def: OrbitDef) {
        todo!()
    }

    fn from_orbit_transformation_unchecked(perm: &[u8], ori: &[u8], orbit_def: OrbitDef) -> Self {
        todo!()
    }

    fn induces_sorted_orbit_cycle_type(
        &self,
        sorted_orbit_cycle_type: &[(NonZeroU8, bool)],
        orbit_def: OrbitDef,
        multi_bv: <Self::MultiBv as MultiBvInterface>::MultiBvReusableRef<'_>,
    ) -> bool {
        todo!()
    }

    fn exact_hash(&self, orbit_def: OrbitDef) -> u64 {
        todo!();
    }
}
