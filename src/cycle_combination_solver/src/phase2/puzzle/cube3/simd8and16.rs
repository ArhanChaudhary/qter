#![cfg_attr(any(simd32, not(simd8and16)), allow(dead_code, unused_variables))]

use super::common::Cube3Interface;
use crate::phase2::puzzle::OrientedPartition;
use std::simd::{cmp::SimdPartialEq, num::SimdInt, u8x16, u8x8};

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Cube3 {
    pub ep: u8x16,
    pub eo: u8x16,
    pub cp: u8x8,
    pub co: u8x8,
}

const EO_MOD_SWIZZLE: u8x16 = u8x16::from_array([0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
const CO_MOD_SWIZZLE: u8x8 = u8x8::from_array([0, 1, 2, 0, 1, 2, 0, 0]);
const TWOS: u8x16 = u8x16::splat(2);
const THREES: u8x8 = u8x8::splat(3);

impl Cube3Interface for Cube3 {
    fn from_sorted_transformations(sorted_transformations: &[Vec<(u8, u8)>]) -> Self {
        let corners_transformation = &sorted_transformations[0];
        let edges_transformation = &sorted_transformations[1];

        let mut ep = u8x16::splat(15);
        let mut eo = u8x16::splat(0);
        let mut cp = u8x8::splat(0);
        let mut co = u8x8::splat(0);

        for i in 0..12 {
            let (perm, orientation_delta) = edges_transformation[i];
            ep[i] = perm;
            eo[i] = orientation_delta;
        }

        for i in 0..8 {
            let (perm, orientation) = corners_transformation[i];
            cp[i] = perm;
            co[i] = orientation;
        }

        Cube3 { ep, eo, cp, co }
    }

    fn replace_compose(&mut self, a: &Self, b: &Self) {
        // FIXME: probably not a big deal, but the armv7 target in swizzle_dyn
        // swizzles high bits as well as low bits and this will be a tiny bit
        // slower than otherwise. May be worth special casing?

        // TODO: bench using sub and min

        // Benchmarking on a 2020 Mac M1 has shown that swizzling twice is
        // faster than taking the modulus (1.93ns vs 3.69ns)
        self.ep = a.ep.swizzle_dyn(b.ep);
        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(a.eo.swizzle_dyn(b.ep) + b.eo);
        // self.eo = (a.eo.swizzle_dyn(b.ep) + b.eo) % TWOS;
        self.cp = a.cp.swizzle_dyn(b.cp);
        self.co = CO_MOD_SWIZZLE.swizzle_dyn(a.co.swizzle_dyn(b.cp) + b.co);
        // self.co = (a.co.swizzle_dyn(b.cp) + b.co) % THREES;
    }

    fn replace_inverse(&mut self, a: &Self) {
        // Permutation inversion taken from Andrew Skalski's vcube[1]. The
        // addition sequence was generated using [2].
        // [1] https://github.com/Voltara/vcube
        // [2] http://wwwhomes.uni-bielefeld.de/achim/addition_chain.html
        //
        // Benchmarked on a 2020 Mac M1: 3.95ns
        //
        // Note that there does not seem to be any speed difference when these
        // instructions are reordered (codegen puts all u8x8 and u8x16 swizzles
        // together).
        let mut pow_3_ep = a.ep.swizzle_dyn(a.ep);
        pow_3_ep = pow_3_ep.swizzle_dyn(a.ep);
        self.ep = pow_3_ep.swizzle_dyn(pow_3_ep);
        self.ep = self.ep.swizzle_dyn(self.ep);
        self.ep = self.ep.swizzle_dyn(self.ep).swizzle_dyn(pow_3_ep);
        self.ep = self.ep.swizzle_dyn(self.ep);
        self.ep = self.ep.swizzle_dyn(self.ep);
        self.ep = self.ep.swizzle_dyn(self.ep);
        self.ep = self.ep.swizzle_dyn(self.ep).swizzle_dyn(a.ep);
        self.ep = self.ep.swizzle_dyn(self.ep);
        self.ep = self.ep.swizzle_dyn(self.ep);
        self.ep = self.ep.swizzle_dyn(self.ep);
        self.ep = self.ep.swizzle_dyn(self.ep);
        self.ep = self.ep.swizzle_dyn(self.ep).swizzle_dyn(pow_3_ep);
        self.ep = self.ep.swizzle_dyn(self.ep).swizzle_dyn(a.ep);

        let mut pow_3_cp = a.cp.swizzle_dyn(a.cp);
        pow_3_cp = pow_3_cp.swizzle_dyn(a.cp);
        self.cp = pow_3_cp.swizzle_dyn(pow_3_cp);
        self.cp = self.cp.swizzle_dyn(self.cp);
        self.cp = self.cp.swizzle_dyn(self.cp).swizzle_dyn(pow_3_cp);
        self.cp = self.cp.swizzle_dyn(self.cp);
        self.cp = self.cp.swizzle_dyn(self.cp);
        self.cp = self.cp.swizzle_dyn(self.cp);
        self.cp = self.cp.swizzle_dyn(self.cp).swizzle_dyn(a.cp);
        self.cp = self.cp.swizzle_dyn(self.cp);
        self.cp = self.cp.swizzle_dyn(self.cp);
        self.cp = self.cp.swizzle_dyn(self.cp);
        self.cp = self.cp.swizzle_dyn(self.cp);
        self.cp = self.cp.swizzle_dyn(self.cp).swizzle_dyn(pow_3_cp);
        self.cp = self.cp.swizzle_dyn(self.cp).swizzle_dyn(a.cp);

        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(TWOS - a.eo).swizzle_dyn(self.ep);
        self.co = CO_MOD_SWIZZLE
            .swizzle_dyn(THREES - a.co)
            .swizzle_dyn(self.cp);
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

impl Cube3 {
    pub fn replace_inverse_brute(&mut self, a: &Self) {
        // Benchmarked on a 2020 Mac M1: 10.19ns
        self.ep = u8x16::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 15, 15, 15]);
        self.cp = u8x8::splat(0);
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
            self.ep |= ep_trial & ep_correct;

            if i < 8 {
                let cp_trial = u8x8::splat(i);
                const CP_IDENTITY: u8x8 = u8x8::from_array([0, 1, 2, 3, 4, 5, 6, 7]);
                let cp_correct: u8x8 =
                    a.cp.swizzle_dyn(cp_trial)
                        .simd_eq(CP_IDENTITY)
                        .to_int()
                        .cast();
                self.cp |= cp_trial & cp_correct;
            }
        }

        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(TWOS - a.eo).swizzle_dyn(self.ep);
        self.co = CO_MOD_SWIZZLE
            .swizzle_dyn(THREES - a.co)
            .swizzle_dyn(self.cp);
    }

    pub fn replace_inverse_raw(&mut self, a: &Self) {
        // Benchmarked on a 2020 Mac M1: 7.16ns

        for i in 0..12 {
            // SAFETY: ep is length 12, so i is always in bounds
            unsafe {
                *self.ep.as_mut_array().get_unchecked_mut(a.ep[i] as usize) = i as u8;
            }
            if i < 8 {
                // SAFETY: cp is length 8, so i is always in bounds
                unsafe {
                    *self.cp.as_mut_array().get_unchecked_mut(a.cp[i] as usize) = i as u8;
                }
            }
        }

        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(TWOS - a.eo).swizzle_dyn(self.ep);
        self.co = CO_MOD_SWIZZLE
            .swizzle_dyn(THREES - a.co)
            .swizzle_dyn(self.cp);
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;
    use crate::phase2::puzzle::{tests::apply_moves, PuzzleDef};
    use puzzle_geometry::ksolve::KPUZZLE_3X3;

    #[test]
    #[cfg_attr(not(simd8and16), ignore)]
    fn test_brute_force_inversion() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let mut result = solved.clone();

        let state_r2_b_prime = apply_moves(&cube3_def, &solved, "R2 B'", 1);
        result.replace_inverse_brute(&state_r2_b_prime);

        let state_b_r2 = apply_moves(&cube3_def, &solved, "B R2", 1);
        assert_eq!(result, state_b_r2);

        let in_r_f_cycle = apply_moves(&cube3_def, &solved, "R F", 40);
        result.replace_inverse_brute(&in_r_f_cycle);

        let remaining_r_f_cycle = apply_moves(&cube3_def, &solved, "R F", 65);
        assert_eq!(result, remaining_r_f_cycle);

        for i in 1..=5 {
            let state = apply_moves(&cube3_def, &solved, "L F L' F'", i);
            result.replace_inverse_brute(&state);
            let remaining_state = apply_moves(&cube3_def, &solved, "L F L' F'", 6 - i);
            assert_eq!(result, remaining_state);
        }

        for _ in 0..100 {
            let mut prev_result = solved.clone();
            let mut result = solved.clone();
            for _ in 0..20 {
                let move_index = fastrand::choice(0_u8..18).unwrap();
                let move_ = &cube3_def.moves[move_index as usize];
                prev_result.replace_compose(&result, &move_.puzzle_state);
                std::mem::swap(&mut result, &mut prev_result);
            }
            prev_result.replace_inverse_brute(&result);
            result.replace_compose(&prev_result, &result.clone());
            assert_eq!(result, solved);
        }
    }

    #[test]
    #[cfg_attr(not(simd8and16), ignore)]
    fn test_raw_inversion() {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let mut result = solved.clone();

        let state_r2_b_prime = apply_moves(&cube3_def, &solved, "R2 B'", 1);
        result.replace_inverse_raw(&state_r2_b_prime);

        let state_b_r2 = apply_moves(&cube3_def, &solved, "B R2", 1);
        assert_eq!(result, state_b_r2);

        let in_r_f_cycle = apply_moves(&cube3_def, &solved, "R F", 40);
        result.replace_inverse_raw(&in_r_f_cycle);

        let remaining_r_f_cycle = apply_moves(&cube3_def, &solved, "R F", 65);
        assert_eq!(result, remaining_r_f_cycle);

        for i in 1..=5 {
            let state = apply_moves(&cube3_def, &solved, "L F L' F'", i);
            result.replace_inverse_raw(&state);
            let remaining_state = apply_moves(&cube3_def, &solved, "L F L' F'", 6 - i);
            assert_eq!(result, remaining_state);
        }

        for _ in 0..100 {
            let mut prev_result = solved.clone();
            let mut result = solved.clone();
            for _ in 0..20 {
                let move_index = fastrand::choice(0_u8..18).unwrap();
                let move_ = &cube3_def.moves[move_index as usize];
                prev_result.replace_compose(&result, &move_.puzzle_state);
                std::mem::swap(&mut result, &mut prev_result);
            }
            prev_result.replace_inverse_raw(&result);
            result.replace_compose(&prev_result, &result.clone());
            assert_eq!(result, solved);
        }
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_brute_force_inversion(b: &mut test::Bencher) {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let mut result = solved.clone();
        let order_1260 = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 100);
        b.iter(|| {
            test::black_box(&mut result).replace_inverse_brute(test::black_box(&order_1260));
        });
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_raw_inversion(b: &mut test::Bencher) {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let mut result = solved.clone();
        let order_1260 = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 100);
        b.iter(|| {
            test::black_box(&mut result).replace_inverse_raw(test::black_box(&order_1260));
        });
    }
}
