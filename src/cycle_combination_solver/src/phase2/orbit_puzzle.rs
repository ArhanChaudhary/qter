use super::puzzle::{MultiBvInterface, OrbitDef};
use std::{hash::Hash, num::NonZeroU8};

pub mod cube3;
pub mod slice_orbit_puzzle;

pub trait OrbitPuzzleState {
    type MultiBv: MultiBvInterface;

    fn replace_compose(&mut self, a: &Self, b: &Self, orbit_def: OrbitDef);
    fn induces_sorted_orbit_cycle_type(
        &self,
        sorted_orbit_cycle_type: &[(NonZeroU8, bool)],
        orbit_def: OrbitDef,
        multi_bv: <Self::MultiBv as MultiBvInterface>::MultiBvReusableRef<'_>,
    ) -> bool;
    fn approximate_hash(&self) -> impl Hash;
    fn exact_hasher(&self, orbit_def: OrbitDef) -> u64;
}

pub trait OrbitPuzzleConstructors {
    type MultiBv: MultiBvInterface;

    fn new_multi_bv(orbit_def: OrbitDef) -> Self::MultiBv;
    fn from_orbit_transformation_unchecked<B: AsRef<[u8]>>(
        perm: B,
        ori: B,
        orbit_def: OrbitDef,
    ) -> Self;
}
