use super::common::CUBE_3_SORTED_ORBIT_DEFS;
use crate::phase2::puzzle::{KSolveConversionError, OrbitDef, OrientedPartition, PuzzleState};
use std::hash::{Hash, Hasher};
use std::simd::{u8x16, u8x8};

#[derive(Clone, Debug)]
pub struct Cube3 {
    pub ep: u8x16,
    pub eo: u8x16,
    pub cp: u8x8,
    pub co: u8x8,
}

impl PartialEq for Cube3 {
    fn eq(&self, other: &Self) -> bool {
        self.ep[..12].eq(&other.ep[..12])
            && self.eo[..12].eq(&other.eo[..12])
            && self.cp.eq(&other.cp)
            && self.co.eq(&other.co)
    }
}

impl Hash for Cube3 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ep[..12].hash(state);
        self.eo[..12].hash(state);
        self.cp.hash(state);
        self.co.hash(state);
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
        let corners_transformation = &sorted_transformations[0];
        let edges_transformation = &sorted_transformations[1];

        let mut ep = u8x16::splat(0);
        let mut eo = u8x16::splat(0);
        let mut cp = u8x8::splat(0);
        let mut co = u8x8::splat(0);

        for (i, &(perm, orientation_delta)) in edges_transformation.iter().enumerate() {
            ep[i] = perm;
            eo[i] = orientation_delta;
        }

        for (i, &(perm, orientation_delta)) in corners_transformation.iter().enumerate() {
            cp[i] = perm;
            co[i] = orientation_delta;
        }

        Cube3 { ep, eo, cp, co }
    }

    fn replace_compose(&mut self, a: &Self, b: &Self, _sorted_orbit_defs: &[OrbitDef]) {
        // Benching from a 2020 Mac M1 has shown that swizzling twice is
        // marginally faster than taking the modulus
        const EO_MOD_SWIZZLE: u8x16 =
            u8x16::from_array([0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        const CO_MOD_SWIZZLE: u8x8 = u8x8::from_array([0, 1, 2, 0, 1, 2, 0, 0]);
        // const TWOS: u8x16 = u8x16::from_array([2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1]);
        // const THREES: u8x8 = u8x8::from_array([3, 3, 3, 3, 3, 3, 3, 3]);
        self.ep = a.ep.swizzle_dyn(b.ep);
        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(a.eo.swizzle_dyn(b.ep) + b.eo);
        // self.eo = (a.eo.swizzle_dyn(b.ep) + b.eo) % TWOS;
        self.cp = a.cp.swizzle_dyn(b.cp);
        self.co = CO_MOD_SWIZZLE.swizzle_dyn(a.co.swizzle_dyn(b.cp) + b.co);
        // self.co = (a.co.swizzle_dyn(b.cp) + b.co) % THREES;
    }

    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: &[OrbitDef]) {
        todo!();
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        mut multi_bv: [u16; 2],
        _sorted_orbit_defs: &[OrbitDef],
    ) -> bool {
        let mut covered_cycles_count = 0_u8;

        // SAFETY: validate_sorted_orbit_defs ensures that sorted_cycle_type.len() == 2
        let sorted_corner_partition = unsafe { sorted_cycle_type.get_unchecked(0) };
        for i in 0..8 {
            if multi_bv[0] & (1 << i) != 0 {
                continue;
            }
            multi_bv[0] |= 1 << i;
            let mut actual_cycle_length = 1;
            // SAFETY: cp is length 8, so i is always in bounds
            let mut corner = unsafe { *self.cp.as_array().get_unchecked(i) } as usize;
            // SAFETY: co is length 8, and corner is always between 0 and 8, so corner is always in bounds
            let mut orientation_sum = unsafe { *self.co.as_array().get_unchecked(corner) };

            while corner != i {
                actual_cycle_length += 1;
                multi_bv[0] |= 1 << corner;
                // SAFETY: cp is length 8, and corner is always between 0 and 8, so corner is always in bounds
                corner = unsafe { *self.cp.as_array().get_unchecked(corner) } as usize;
                // SAFETY: co is length 8, and corner is always between 0 and 8, so corner is always in bounds
                orientation_sum += unsafe { *self.co.as_array().get_unchecked(corner) };
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
        // SAFETY: validate_sorted_orbit_defs ensures that sorted_cycle_type.len() == 2
        let sorted_edge_partition = unsafe { sorted_cycle_type.get_unchecked(1) };
        for i in 0..12 {
            if multi_bv[0] & (1 << i) != 0 {
                continue;
            }
            multi_bv[0] |= 1 << i;
            let mut actual_cycle_length = 1;
            // SAFETY: ep is length 12, so i is always in bounds
            let mut edge = unsafe { *self.ep.as_array().get_unchecked(i) } as usize;
            // SAFETY: eo is length 12, and edge is always between 0 and 12, so edge is always in bounds
            let mut orientation_sum = unsafe { *self.eo.as_array().get_unchecked(edge) };

            while edge != i {
                actual_cycle_length += 1;
                multi_bv[0] |= 1 << edge;
                // SAFETY: ep is length 12, and edge is always between 0 and 12, so edge is always in bounds
                edge = unsafe { *self.ep.as_array().get_unchecked(edge) } as usize;
                // SAFETY: eo is length 12, and edge is always between 0 and 12, so edge is always in bounds
                orientation_sum += unsafe { *self.eo.as_array().get_unchecked(edge) };
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
