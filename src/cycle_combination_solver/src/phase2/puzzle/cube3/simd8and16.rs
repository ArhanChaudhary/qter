#![cfg_attr(any(simd32, not(simd8and16)), allow(dead_code, unused_variables))]

use super::common::Cube3Interface;
use crate::phase2::puzzle::OrientedPartition;
use std::hash::{Hash, Hasher};
use std::simd::{u8x16, u8x8};

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Cube3 {
    pub ep: u8x16,
    pub eo: u8x16,
    pub cp: u8x8,
    pub co: u8x8,
}

// TODO: probably not a big deal, but the armv7 target in swizzle_dyn swizzles
// high bits as well as low bits and this will be a tiny bit slower than
// otherwise. May be worth special casing?

const EO_MOD_SWIZZLE: u8x16 = u8x16::from_array([0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
const CO_MOD_SWIZZLE: u8x8 = u8x8::from_array([0, 1, 2, 0, 1, 2, 0, 0]);
const TWOS: u8x16 = u8x16::splat(2);
const THREES: u8x8 = u8x8::splat(3);

impl Cube3Interface for Cube3 {
    fn from_sorted_transformations_unchecked(sorted_transformations: &[Vec<(u8, u8)>]) -> Self {
        let corners_transformation = &sorted_transformations[0];
        let edges_transformation = &sorted_transformations[1];

        let mut ep = u8x16::splat(0);
        let mut eo = u8x16::splat(0);
        let mut cp = u8x8::splat(0);
        let mut co = u8x8::splat(0);

        for i in 0..12 {
            let (perm, orientation_delta) = edges_transformation[i];
            ep[i] = perm;
            eo[i] = orientation_delta;
        }

        ep[12..16].fill(15);

        for i in 0..8 {
            let (perm, orientation) = corners_transformation[i];
            cp[i] = perm;
            co[i] = orientation;
        }

        Cube3 { ep, eo, cp, co }
    }

    fn replace_compose(&mut self, a: &Self, b: &Self) {
        // Benching from a 2020 Mac M1 has shown that swizzling twice is
        // faster than taking the modulus (3.07ns vs 5.94ns)
        self.ep = a.ep.swizzle_dyn(b.ep);
        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(a.eo.swizzle_dyn(b.ep) + b.eo);
        // self.eo = (a.eo.swizzle_dyn(b.ep) + b.eo) % TWOS;
        self.cp = a.cp.swizzle_dyn(b.cp);
        self.co = CO_MOD_SWIZZLE.swizzle_dyn(a.co.swizzle_dyn(b.cp) + b.co);
        // self.co = (a.co.swizzle_dyn(b.cp) + b.co) % THREES;
    }

    fn replace_inverse(&mut self, a: &Self) {
        let mut ep_inverse;
        let mut cp_inverse;

        // Three ways to inverse permutation, benched on a 2020 Mac M1. These
        // results are probably not the case on all platforms, experimentation
        // is encouraged.

        #[cfg(true)]
        // 6.36ns
        {
            // Permutation inversion taken from Andrew Skalski's vcube[1]. The
            // addition sequence was generated using [2].
            // [1] https://github.com/Voltara/vcube
            // [2] http://wwwhomes.uni-bielefeld.de/achim/addition_chain.html
            //
            // Note that there does not seem to be any speed difference when these
            // instructions are reordered (codegen puts all u8x8 and u8x16 swizzles
            // together).
            let mut pow_3_ep = a.ep.swizzle_dyn(a.ep);
            pow_3_ep = pow_3_ep.swizzle_dyn(a.ep);
            ep_inverse = pow_3_ep.swizzle_dyn(pow_3_ep);
            for _ in 0..2 {
                ep_inverse = ep_inverse.swizzle_dyn(ep_inverse);
            }
            ep_inverse = ep_inverse.swizzle_dyn(pow_3_ep);
            for _ in 0..4 {
                ep_inverse = ep_inverse.swizzle_dyn(ep_inverse);
            }
            ep_inverse = ep_inverse.swizzle_dyn(a.ep);
            for _ in 0..5 {
                ep_inverse = ep_inverse.swizzle_dyn(ep_inverse);
            }
            ep_inverse = ep_inverse.swizzle_dyn(pow_3_ep);
            ep_inverse = ep_inverse.swizzle_dyn(ep_inverse);
            ep_inverse = ep_inverse.swizzle_dyn(a.ep);

            let mut pow_3_cp = a.cp.swizzle_dyn(a.cp);
            pow_3_cp = pow_3_cp.swizzle_dyn(a.cp);
            cp_inverse = pow_3_cp.swizzle_dyn(pow_3_cp);
            for _ in 0..2 {
                cp_inverse = cp_inverse.swizzle_dyn(cp_inverse);
            }
            cp_inverse = cp_inverse.swizzle_dyn(pow_3_cp);
            for _ in 0..4 {
                cp_inverse = cp_inverse.swizzle_dyn(cp_inverse);
            }
            cp_inverse = cp_inverse.swizzle_dyn(a.cp);
            for _ in 0..5 {
                cp_inverse = cp_inverse.swizzle_dyn(cp_inverse);
            }
            cp_inverse = cp_inverse.swizzle_dyn(pow_3_cp);
            cp_inverse = cp_inverse.swizzle_dyn(cp_inverse);
            cp_inverse = cp_inverse.swizzle_dyn(a.cp);
        }
        // TODO: make these benches
        #[cfg(false)]
        // 9.68ns
        {
            use std::simd::cmp::SimdPartialEq;
            use std::simd::num::SimdInt;

            ep_inverse = u8x16::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 15, 15, 15]);
            cp_inverse = u8x8::splat(0);
            // Brute force the inverse by checking all possible values and
            // using a mask to check when equal to identity (also inspired by
            // Andrew Skalski's vcube).
            for i in 0..12 {
                let ep_trial = u8x16::splat(i);
                const EP_IDENTITY: u8x16 =
                    u8x16::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 15, 15, 15, 15]);
                let ep_correct: u8x16 =
                    a.ep.swizzle_dyn(ep_trial)
                        .simd_eq(EP_IDENTITY)
                        .to_int()
                        .cast();
                ep_inverse |= ep_trial & ep_correct;

                if i < 8 {
                    let cp_trial = u8x8::splat(i);
                    const CP_IDENTITY: u8x8 = u8x8::from_array([0, 1, 2, 3, 4, 5, 6, 7]);
                    let cp_correct: u8x8 =
                        a.cp.swizzle_dyn(cp_trial)
                            .simd_eq(CP_IDENTITY)
                            .to_int()
                            .cast();
                    cp_inverse |= cp_trial & cp_correct;
                }
            }
        }
        #[cfg(false)]
        // 11.7ns
        {
            // Sanity check that SIMD is actually faster.
            ep_inverse = self.ep;
            cp_inverse = self.cp;

            for i in 0..12 {
                // SAFETY: ep is length 12, so i is always in bounds
                unsafe {
                    *ep_inverse
                        .as_mut_array()
                        .get_unchecked_mut(a.ep[i] as usize) = i as u8;
                }
                if i < 8 {
                    // SAFETY: cp is length 8, so i is always in bounds
                    unsafe {
                        *cp_inverse
                            .as_mut_array()
                            .get_unchecked_mut(a.cp[i] as usize) = i as u8;
                    }
                }
            }
        }

        let eo_inverse = EO_MOD_SWIZZLE
            .swizzle_dyn(TWOS - a.eo)
            .swizzle_dyn(ep_inverse);
        let co_inverse = CO_MOD_SWIZZLE
            .swizzle_dyn(THREES - a.co)
            .swizzle_dyn(cp_inverse);

        self.ep = ep_inverse;
        self.eo = eo_inverse;
        self.cp = cp_inverse;
        self.co = co_inverse;
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        mut multi_bv: [u16; 2],
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

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;
    use crate::phase2::puzzle::tests::*;
    use test::Bencher;

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_compose(b: &mut Bencher) {
        bench_compose_helper::<Cube3>(b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_inverse(b: &mut Bencher) {
        bench_inverse_helper::<Cube3>(b);
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_induces_sorted_cycle_type(b: &mut Bencher) {
        bench_induces_sorted_cycle_type_helper::<Cube3>(b);
    }
}
