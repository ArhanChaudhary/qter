#[cfg(not(any(avx2, simd8and16)))]
pub type Cube3 = super::StackPuzzle<40>;

mod common {
    use crate::phase2::puzzle::{KSolveConversionError, OrbitDef, OrientedPartition, PuzzleState};
    use std::{fmt::Debug, hash::Hash, num::NonZeroU8};

    pub trait Cube3Interface: Hash + Clone + PartialEq + Debug {
        fn from_sorted_transformations(sorted_transformations: &[Vec<(u8, u8)>]) -> Self;
        fn replace_compose(&mut self, a: &Self, b: &Self);
        fn replace_inverse(&mut self, a: &Self);
        fn induces_sorted_cycle_type(&self, sorted_cycle_type: &[OrientedPartition; 2]) -> bool;
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

        fn new_multi_bv(_sorted_orbit_defs: &[OrbitDef]) {}

        fn try_from_transformation_meta(
            sorted_transformations: &[Vec<(u8, u8)>],
            sorted_orbit_defs: &[OrbitDef],
        ) -> Result<C, KSolveConversionError> {
            if sorted_orbit_defs == CUBE_3_SORTED_ORBIT_DEFS {
                Ok(Self::from_sorted_transformations(sorted_transformations))
            } else {
                Err(KSolveConversionError::InvalidOrbitDefs(
                    CUBE_3_SORTED_ORBIT_DEFS.to_vec(),
                    sorted_orbit_defs.to_vec(),
                ))
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
    }
}

pub(in crate::phase2::puzzle) mod avx2;
pub(in crate::phase2::puzzle) mod simd8and16;

#[cfg(avx2)]
pub use avx2::Cube3;

#[cfg(all(not(avx2), simd8and16))]
pub use simd8and16::Cube3;

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
