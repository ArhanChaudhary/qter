#[cfg(not(any(simd32, simd8and16)))]
pub type Cube3 = super::StackPuzzle<40>;

mod common {
    use crate::phase2::puzzle::{KSolveConversionError, OrbitDef, OrientedPartition, PuzzleState};
    use std::{fmt::Debug, hash::Hash, num::NonZeroU8};

    pub trait Cube3Interface: Hash + Clone + PartialEq + Debug {
        fn from_sorted_transformations_unchecked(sorted_transformations: &[Vec<(u8, u8)>]) -> Self;
        fn replace_compose(&mut self, a: &Self, b: &Self);
        fn replace_inverse(&mut self, a: &Self);
        fn induces_sorted_cycle_type(
            &self,
            sorted_cycle_type: &[OrientedPartition],
            multi_bv: [u16; 2],
        ) -> bool;
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
        type MultiBv = [u16; 2];

        fn new_multi_bv(_sorted_orbit_defs: &[OrbitDef]) -> [u16; 2] {
            Default::default()
        }

        fn validate_sorted_orbit_defs(
            sorted_orbit_defs: &[OrbitDef],
        ) -> Result<(), KSolveConversionError> {
            if sorted_orbit_defs == CUBE_3_SORTED_ORBIT_DEFS {
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
            sorted_orbit_defs: &[OrbitDef],
        ) -> Self {
            debug_assert!(sorted_transformations.len() == 2);
            debug_assert_eq!(sorted_transformations[0].len(), 8);
            debug_assert_eq!(sorted_transformations[1].len(), 12);
            debug_assert!(Self::validate_sorted_orbit_defs(sorted_orbit_defs).is_ok());
            Self::from_sorted_transformations_unchecked(sorted_transformations)
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
            multi_bv: [u16; 2],
            _sorted_orbit_defs: &[OrbitDef],
        ) -> bool {
            self.induces_sorted_cycle_type(sorted_cycle_type, multi_bv)
        }
    }
}

pub(in crate::phase2::puzzle) mod simd32;
pub(in crate::phase2::puzzle) mod simd8and16;

#[cfg(simd32)]
pub use simd32::Cube3;

#[cfg(all(not(simd32), simd8and16))]
pub use simd8and16::Cube3;
