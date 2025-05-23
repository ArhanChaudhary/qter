#[cfg(not(any(avx2, simd8and16)))]
pub type Cube3 = super::slice_puzzle::StackPuzzle<40>;

mod common {
    use crate::phase2::puzzle::{KSolveConversionError, OrbitDef, OrientedPartition, PuzzleState};
    use std::hash::Hash;
    use std::{fmt::Debug, num::NonZeroU8};

    pub trait Cube3Interface: Clone + PartialEq + Debug {
        fn from_sorted_transformations(sorted_transformations: &[Vec<(u8, u8)>]) -> Self;
        fn replace_compose(&mut self, a: &Self, b: &Self);
        fn replace_inverse(&mut self, a: &Self);
        fn induces_sorted_cycle_type(&self, sorted_cycle_type: &[OrientedPartition; 2]) -> bool;
        fn orbit_bytes(&self, orbit_index: usize) -> ([u8; 16], [u8; 16]);
        fn exact_hasher_orbit(&self, orbit_index: usize) -> u64;
        fn approximate_hash_orbit(&self, orbit_index: usize) -> impl Hash;
    }

    const CUBE_3_SORTED_ORBIT_DEFS: [OrbitDef; 2] = [
        OrbitDef {
            piece_count: NonZeroU8::new(8).unwrap(),
            orientation_count: NonZeroU8::new(3).unwrap(),
        },
        OrbitDef {
            piece_count: NonZeroU8::new(12).unwrap(),
            orientation_count: NonZeroU8::new(2).unwrap(),
        },
    ];

    impl<C: Cube3Interface> PuzzleState for C {
        type MultiBv = ();
        type OrbitBytesBuf<'a>
            = [u8; 16]
        where
            Self: 'a;

        fn new_multi_bv(_sorted_orbit_defs: &[OrbitDef]) {}

        fn try_from_transformation_meta(
            sorted_transformations: &[Vec<(u8, u8)>],
            sorted_orbit_defs: &[OrbitDef],
        ) -> Result<C, KSolveConversionError> {
            if sorted_orbit_defs == CUBE_3_SORTED_ORBIT_DEFS {
                Ok(Self::from_sorted_transformations(sorted_transformations))
            } else {
                Err(KSolveConversionError::InvalidOrbitDefs {
                    expected: CUBE_3_SORTED_ORBIT_DEFS.to_vec(),
                    actual: sorted_orbit_defs.to_vec(),
                })
            }
        }

        fn replace_compose(&mut self, a: &Self, b: &Self, _sorted_orbit_defs: &[OrbitDef]) {
            self.replace_compose(a, b);
        }

        fn replace_inverse(&mut self, a: &Self, _sorted_orbit_defs: &[OrbitDef]) {
            self.replace_inverse(a);
        }

        fn induces_sorted_cycle_type(
            &self,
            sorted_cycle_type: &[OrientedPartition],
            _sorted_orbit_defs: &[OrbitDef],
            _multi_bv: (),
        ) -> bool {
            // SAFETY: `try_from_transformation_meta` guarantees that this will
            // always be length 2
            self.induces_sorted_cycle_type(unsafe {
                sorted_cycle_type.try_into().unwrap_unchecked()
            })
        }

        fn next_orbit_identifer(orbit_identifier: usize, _orbit_def: OrbitDef) -> usize {
            orbit_identifier + 1
        }

        fn orbit_bytes(
            &self,
            orbit_identifier: usize,
            _orbit_def: OrbitDef,
        ) -> ([u8; 16], [u8; 16]) {
            self.orbit_bytes(orbit_identifier)
        }

        fn exact_hasher_orbit(&self, orbit_identifier: usize, _orbit_def: OrbitDef) -> u64 {
            self.exact_hasher_orbit(orbit_identifier)
        }

        fn approximate_hash_orbit(
            &self,
            orbit_identifier: usize,
            _orbit_def: OrbitDef,
        ) -> impl Hash {
            self.approximate_hash_orbit(orbit_identifier)
        }
    }
}

pub(in crate::phase2::puzzle) mod avx2;
pub(in crate::phase2::puzzle) mod simd8and16;

#[cfg(avx2)]
pub use avx2::Cube3;

#[cfg(all(not(avx2), simd8and16))]
pub use simd8and16::UncompressedCube3 as Cube3;

// TODO: avx512vl when we have time

// TODO: NxN cubes:

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
