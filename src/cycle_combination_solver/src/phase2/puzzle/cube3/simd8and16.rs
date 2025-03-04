#![cfg_attr(any(avx2, not(simd8and16)), allow(dead_code, unused_variables))]

use super::common::Cube3Interface;
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

        let mut ep = u8x16::splat(0);
        let mut eo = u8x16::splat(0);
        let mut cp = u8x8::splat(0);
        let mut co = u8x8::splat(0);

        for i in 0..12 {
            let (perm, orientation_delta) = edges_transformation[i];
            ep[i] = perm;
            eo[i] = orientation_delta;
        }

        ep[12] = 12;
        ep[13] = 13;
        ep[14] = 14;
        ep[15] = 15;

        for i in 0..8 {
            let (perm, orientation) = corners_transformation[i];
            cp[i] = perm;
            co[i] = orientation;
        }

        Cube3 { ep, eo, cp, co }
    }

    fn replace_compose(&mut self, a: &Self, b: &Self) {
        // Benchmarked on a 2020 Mac M1: 1.93ns
        self.ep = a.ep.swizzle_dyn(b.ep);
        self.eo = EO_MOD_SWIZZLE.swizzle_dyn(a.eo.swizzle_dyn(b.ep) + b.eo);
        self.cp = a.cp.swizzle_dyn(b.cp);
        self.co = CO_MOD_SWIZZLE.swizzle_dyn(a.co.swizzle_dyn(b.cp) + b.co);
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

    fn ep_eo_cp_co(
        &self,
        ep: &mut [u8; 16],
        eo: &mut [u8; 16],
        cp: &mut [u8; 8],
        co: &mut [u8; 8],
    ) {
        self.ep.copy_to_slice(ep);
        self.eo.copy_to_slice(eo);
        self.cp.copy_to_slice(cp);
        self.co.copy_to_slice(co);
    }
}

impl Cube3 {
    pub fn replace_inverse_brute(&mut self, a: &Self) {
        // Benchmarked on a 2020 Mac M1: 6.01ns
        self.ep = u8x16::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 13, 14, 15]);
        self.cp = u8x8::splat(0);
        // Brute force the inverse by checking all possible values and
        // using a mask to check when equal to identity (also inspired by
        // Andrew Skalski's vcube).
        const EP_IDENTITY: u8x16 =
            u8x16::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        const CP_IDENTITY: u8x8 = u8x8::from_array([0, 1, 2, 3, 4, 5, 6, 7]);
        macro_rules! brute_unroll {
            ($i:literal) => {
                let ep_trial = u8x16::splat($i);
                let ep_correct: u8x16 =
                    a.ep.swizzle_dyn(ep_trial)
                        .simd_eq(EP_IDENTITY)
                        .to_int()
                        .cast();
                self.ep |= ep_trial & ep_correct;

                // Note that doing simd16 and simd8 stuff separately isn't any
                // faster
                if $i < 8 {
                    let cp_trial = u8x8::splat($i);
                    let cp_correct: u8x8 =
                        a.cp.swizzle_dyn(cp_trial)
                            .simd_eq(CP_IDENTITY)
                            .to_int()
                            .cast();
                    self.cp |= cp_trial & cp_correct;
                }
            };
        }

        brute_unroll!(0);
        brute_unroll!(1);
        brute_unroll!(2);
        brute_unroll!(3);
        brute_unroll!(4);
        brute_unroll!(5);
        brute_unroll!(6);
        brute_unroll!(7);
        brute_unroll!(8);
        brute_unroll!(9);
        brute_unroll!(10);
        brute_unroll!(11);

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
