use crate::orbit_puzzle::{OrbitPuzzleStateImplementor, SpecializedOrbitPuzzleState};
use std::{hint::unreachable_unchecked, num::NonZeroU8};

#[derive(PartialEq, Clone, Hash)]
pub struct CubeNCorners;

impl SpecializedOrbitPuzzleState for CubeNCorners {
    unsafe fn from_implementor_enum_unchecked(
        implementor_enum: &OrbitPuzzleStateImplementor,
    ) -> &Self {
        match implementor_enum {
            OrbitPuzzleStateImplementor::CubeNCorners(c) => c,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    unsafe fn from_orbit_transformation_unchecked<B: AsRef<[u8]>>(perm: B, ori: B) -> Self {
        todo!()
    }

    fn replace_compose(&mut self, a: &Self, b: &Self) {
        todo!()
    }

    fn induces_sorted_cycle_type(&self, sorted_cycle_type_orbit: &[(NonZeroU8, bool)]) -> bool {
        todo!()
    }

    fn exact_hasher(&self) -> u64 {
        todo!()
    }

    fn approximate_hash(&self) -> &Self {
        self
    }
}
