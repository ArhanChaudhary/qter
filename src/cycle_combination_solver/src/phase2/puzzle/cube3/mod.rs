#[cfg(not(any(avx2, simd8and16)))]
pub type Cube3 = super::StackPuzzle<40>;

mod common {
    use crate::phase2::puzzle::{KSolveConversionError, OrbitDef, OrientedPartition, PuzzleState};
    use std::{fmt::Debug, hash::Hash, hint::assert_unchecked, num::NonZeroU8};

    pub trait Cube3Interface: Hash + Clone + PartialEq + Debug {
        fn from_sorted_transformations(sorted_transformations: &[Vec<(u8, u8)>]) -> Self;
        fn replace_compose(&mut self, a: &Self, b: &Self);
        fn replace_inverse(&mut self, a: &Self);
        fn ep_eo_cp_co(
            &self,
            ep: &mut [u8; 16],
            eo: &mut [u8; 16],
            cp: &mut [u8; 8],
            co: &mut [u8; 8],
        );
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
            mut multi_bv: [u16; 2],
            _sorted_orbit_defs: &[OrbitDef],
        ) -> bool {
            let mut ep = [0; 16];
            let mut eo = [0; 16];
            let mut cp = [0; 8];
            let mut co = [0; 8];
            self.ep_eo_cp_co(&mut ep, &mut eo, &mut cp, &mut co);

            let mut covered_cycles_count = 0_u8;

            // SAFETY: validate_sorted_orbit_defs ensures that sorted_cycle_type.len() == 2
            unsafe {
                assert_unchecked(sorted_cycle_type.len() == 2);
            }
            let sorted_corner_partition = &sorted_cycle_type[0];
            for i in 0..8 {
                if multi_bv[0] & (1 << i) != 0 {
                    continue;
                }
                multi_bv[0] |= 1 << i;
                let mut actual_cycle_length = 1;
                // SAFETY: cp is length 8, so i is always in bounds
                let mut corner = unsafe { *cp.get_unchecked(i) } as usize;
                // SAFETY: co is length 8, and corner is always between 0 and 8, so corner is always in bounds
                let mut orientation_sum = unsafe { *co.get_unchecked(corner) };

                while corner != i {
                    actual_cycle_length += 1;
                    multi_bv[0] |= 1 << corner;
                    // SAFETY: cp is length 8, and corner is always between 0 and 8, so corner is always in bounds
                    corner = unsafe { *cp.get_unchecked(corner) } as usize;
                    // SAFETY: co is length 8, and corner is always between 0 and 8, so corner is always in bounds
                    orientation_sum += unsafe { *co.get_unchecked(corner) };
                }

                let actual_orients = orientation_sum % 3 != 0;
                if actual_cycle_length == 1 && !actual_orients {
                    continue;
                }
                let Some(valid_cycle_index) = sorted_corner_partition.iter().enumerate().position(
                    |(j, &(expected_cycle_length, expected_orients))| {
                        expected_cycle_length.get() == actual_cycle_length
                            && expected_orients == actual_orients
                            && (multi_bv[1] & (1 << j) == 0)
                    },
                ) else {
                    return false;
                };
                multi_bv[1] |= 1 << valid_cycle_index;
                covered_cycles_count += 1;
                // cannot possibly return true if this runs
                if covered_cycles_count > sorted_corner_partition.len() as u8 {
                    return false;
                }
            }
            if covered_cycles_count != sorted_corner_partition.len() as u8 {
                return false;
            }

            multi_bv = [0; 2];
            covered_cycles_count = 0;
            let sorted_edge_partition = &sorted_cycle_type[1];
            for i in 0..12 {
                if multi_bv[0] & (1 << i) != 0 {
                    continue;
                }
                multi_bv[0] |= 1 << i;
                let mut actual_cycle_length = 1;
                // SAFETY: ep is length 16, so i is always in bounds
                let mut edge = unsafe { *ep.get_unchecked(i) } as usize;
                // SAFETY: eo is length 16, and edge is always between 0 and 12, so edge is always in bounds
                let mut orientation_sum = unsafe { *eo.get_unchecked(edge) };

                while edge != i {
                    actual_cycle_length += 1;
                    multi_bv[0] |= 1 << edge;
                    // SAFETY: ep is length 16, and edge is always between 0 and 12, so edge is always in bounds
                    edge = unsafe { *ep.get_unchecked(edge) } as usize;
                    // SAFETY: eo is length 16, and edge is always between 0 and 12, so edge is always in bounds
                    orientation_sum += unsafe { *eo.get_unchecked(edge) };
                }

                let actual_orients = orientation_sum % 2 != 0;
                if actual_cycle_length == 1 && !actual_orients {
                    continue;
                }
                let Some(valid_cycle_index) = sorted_edge_partition.iter().enumerate().position(
                    |(j, &(expected_cycle_length, expected_orients))| {
                        expected_cycle_length.get() == actual_cycle_length
                            && expected_orients == actual_orients
                            && (multi_bv[1] & (1 << j) == 0)
                    },
                ) else {
                    return false;
                };
                multi_bv[1] |= 1 << valid_cycle_index;
                covered_cycles_count += 1;
                // cannot possibly return true if this runs
                if covered_cycles_count > sorted_edge_partition.len() as u8 {
                    return false;
                }
            }
            covered_cycles_count == sorted_edge_partition.len() as u8
        }
    }
}

pub(in crate::phase2::puzzle) mod avx2;
pub(in crate::phase2::puzzle) mod simd8and16;

#[cfg(avx2)]
pub use avx2::Cube3;

#[cfg(all(not(avx2), simd8and16))]
pub use simd8and16::Cube3;

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
