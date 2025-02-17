use super::{KSolveConversionError, OrientedPartition, PuzzleState};
use crate::phase2::puzzle::OrbitDef;
use std::{
    simd::{u8x16, u8x32, u8x8},
    sync::LazyLock,
};

static CUBE_3_SORTED_ORBIT_DEFS: LazyLock<Vec<OrbitDef>> = LazyLock::new(|| {
    vec![
        OrbitDef {
            piece_count: 8.try_into().unwrap(),
            orientation_count: 3.try_into().unwrap(),
        },
        OrbitDef {
            piece_count: 12.try_into().unwrap(),
            orientation_count: 2.try_into().unwrap(),
        },
    ]
});

#[cfg(not(any(simd32, simd8and16)))]
pub type Cube3 = super::StackPuzzle<40>;

#[cfg(all(not(simd32), simd8and16))]
#[derive(Clone, Debug)]
pub struct Cube3 {
    pub ep: u8x16,
    pub eo: u8x16,
    pub cp: u8x8,
    pub co: u8x8,
}

#[cfg(simd32)]
#[derive(Clone, Debug)]
pub struct Cube3 {
    perm: u8x32,
    ori: u8x32,
}

#[cfg(any(simd32, simd8and16))]
impl PartialEq for Cube3 {
    #[cfg(not(simd32))]
    fn eq(&self, other: &Self) -> bool {
        self.ep[..12].eq(&other.ep[..12])
            && self.eo[..12].eq(&other.eo[..12])
            && self.cp.eq(&other.cp)
            && self.co.eq(&other.co)
    }

    #[cfg(simd32)]
    fn eq(&self, other: &Self) -> bool {
        // TODO: probably incorrect
        self.perm.eq(&other.perm) && self.ori.eq(&other.ori)
    }
}

#[cfg(any(simd32, simd8and16))]
impl PuzzleState for Cube3 {
    type MultiBv = [u16; 2];

    fn default_multi_bv(_sorted_orbit_defs: &[OrbitDef]) -> [u16; 2] {
        Default::default()
    }

    fn expected_sorted_orbit_defs() -> Option<&'static [OrbitDef]> {
        Some(CUBE_3_SORTED_ORBIT_DEFS.as_slice())
    }

    #[cfg(not(simd32))]
    fn from_sorted_transformations_unchecked(
        sorted_transformations: &[Vec<(u8, u8)>],
        _sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError> {
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

        Ok(Cube3 { ep, eo, cp, co })
    }

    #[cfg(not(simd32))]
    fn replace_compose(&mut self, a: &Self, b: &Self, _sorted_orbit_defs: &[OrbitDef]) {
        const EO_MOD_SWIZZLE: u8x16 =
            u8x16::from_array([0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        const CO_MOD_SWIZZLE: u8x8 = u8x8::from_array([0, 1, 2, 0, 1, 2, 0, 0]);
        // TODO: it is unclear for now if it will later be more efficient or
        // not to combine orientation/permutation into a single simd vector
        self.ep = a.ep.swizzle_dyn(b.ep);
        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(a.eo.swizzle_dyn(b.ep) + b.eo);
        // self.eo = (a.eo.swizzle_dyn(b.ep) + b.eo) % TWOS;
        self.cp = a.cp.swizzle_dyn(b.cp);
        self.co = CO_MOD_SWIZZLE.swizzle_dyn(a.co.swizzle_dyn(b.cp) + b.co);
        // self.co = (a.co.swizzle_dyn(b.cp) + b.co) % THREES;
    }

    #[cfg(not(simd32))]
    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        mut multi_bv: [u16; 2],
        _sorted_orbit_defs: &[OrbitDef],
    ) -> bool {
        let mut covered_cycles_count = 0_u8;

        assert!(sorted_cycle_type.len() == 2);

        let sorted_corner_partition = &sorted_cycle_type[0];
        for i in 0..8 {
            if multi_bv[0] & (1 << i) != 0 {
                continue;
            }
            multi_bv[0] |= 1 << i;
            let mut actual_cycle_length = 1;
            let mut corner = self.cp[i] as usize;
            let mut orientation_sum = self.co[corner];

            while corner != i {
                actual_cycle_length += 1;
                multi_bv[0] |= 1 << corner;
                corner = self.cp[corner] as usize;
                orientation_sum += self.co[corner];
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
            let mut edge = self.ep[i] as usize;
            let mut orientation_sum = self.eo[edge];

            while edge != i {
                actual_cycle_length += 1;
                multi_bv[0] |= 1 << edge;
                edge = self.ep[edge] as usize;
                orientation_sum += self.eo[edge];
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

    #[cfg(simd32)]
    fn from_sorted_transformations_unchecked(
        sorted_transformations: &[Vec<(u8, u8)>],
        _sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError> {
        todo!();
    }

    #[cfg(simd32)]
    fn replace_compose(&mut self, a: &Self, b: &Self, _sorted_orbit_defs: &[OrbitDef]) {
        todo!();
    }

    #[cfg(simd32)]
    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        multi_bv: [u16; 2],
        _sorted_orbit_defs: &[OrbitDef],
    ) -> bool {
        todo!();
    }
}

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
