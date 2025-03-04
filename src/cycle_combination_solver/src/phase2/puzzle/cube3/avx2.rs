#![cfg_attr(not(avx2), allow(dead_code, unused_variables))]

use super::common::Cube3Interface;
#[cfg(all(avx2, target_arch = "x86"))]
use core::arch::x86::_mm256_shuffle_epi8;
#[cfg(all(avx2, target_arch = "x86_64"))]
use core::arch::x86_64::_mm256_shuffle_epi8;
use std::{
    fmt,
    hash::{Hash, Hasher},
    simd::{
        cmp::{SimdOrd, SimdPartialEq},
        num::SimdInt,
        u8x32,
    },
};

#[derive(Clone)]
#[repr(transparent)]
pub struct Cube3(u8x32);

impl PartialEq for Cube3 {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        #[cfg(avx2)]
        extern "vectorcall" fn eq_vectorcall(a: &Cube3, b: &Cube3) -> bool {
            a.0 == b.0
        }
        #[cfg(not(avx2))]
        fn eq_vectorcall(a: &Cube3, b: &Cube3) -> bool {
            unimplemented!()
        }
        eq_vectorcall(self, other)
    }
}

impl Hash for Cube3 {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        #[cfg(avx2)]
        extern "vectorcall" fn hash_vectorcall<H: Hasher>(a: &Cube3, state: &mut H) {
            a.0.hash(state)
        }
        #[cfg(not(avx2))]
        fn hash_vectorcall<H: Hasher>(a: &Cube3, state: &mut H) {
            unimplemented!()
        }
        hash_vectorcall(self, state)
    }
}

impl fmt::Debug for Cube3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ep = [0; 16];
        let mut eo = [0; 16];
        let mut cp = [0; 16];
        let mut co = [0; 16];

        for i in 0..16 {
            ep[i] = self.0[i] & 0b1111;
            eo[i] = self.0[i] >> 4;
        }

        for i in 16..32 {
            cp[i - 16] = self.0[i] & 0b111;
            co[i - 16] = self.0[i] >> 4;
        }

        f.debug_struct("Cube3")
            .field("ep", &ep)
            .field("eo", &eo)
            .field("cp", &cp)
            .field("co", &co)
            .finish()
    }
}

const PERM_MASK: u8x32 = u8x32::splat(0b1111);
const ORI_CARRY_INVERSE: u8x32 = u8x32::from_array([
    0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10,
    0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30,
]);

#[inline(always)]
fn avx2_swizzle_lo(a: u8x32, b: u8x32) -> u8x32 {
    #[cfg(avx2)]
    // SAFETY: a and b are well defined. Honestly not sure why this is unsafe
    unsafe {
        _mm256_shuffle_epi8(a.into(), b.into()).into()
    }
    #[cfg(not(avx2))]
    unimplemented!()
}

impl Cube3Interface for Cube3 {
    fn from_sorted_transformations(sorted_transformations: &[Vec<(u8, u8)>]) -> Self {
        let corners_transformation = &sorted_transformations[0];
        let edges_transformation = &sorted_transformations[1];

        let mut cube = u8x32::from_array([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 13, 14, 15, 0, 0, 0, 0, 0, 0, 0, 0, 8, 9, 10,
            11, 12, 13, 14, 15,
        ]);

        for i in 0..12 {
            let (perm, ori) = edges_transformation[i];
            cube[i] = perm | (ori << 4);
        }

        for i in 16..24 {
            let (perm, ori) = corners_transformation[i - 16];
            cube[i] = perm | (ori << 4);
        }

        Cube3(cube)
    }

    #[inline(always)]
    fn replace_compose(&mut self, a: &Self, b: &Self) {
        // Benchmarked on a 2x Intel Xeon E5-2667 v3 VM: 1.55ns
        fn inner(dst: &mut Cube3, a: &Cube3, b: &Cube3) {
            const ORI_MASK: u8x32 = u8x32::splat(0b11_0000);
            const ORI_CARRY: u8x32 = u8x32::from_array([
                0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
                0x20, 0x20, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30,
                0x30, 0x30, 0x30, 0x30,
            ]);

            let mut composed = avx2_swizzle_lo(a.0, b.0);
            composed += b.0 & ORI_MASK;
            composed = composed.simd_min(composed - ORI_CARRY);

            dst.0 = composed;
        }
        #[cfg(avx2)]
        extern "vectorcall" fn replace_compose_vectorcall(dst: &mut Cube3, a: &Cube3, b: &Cube3) {
            inner(dst, a, b)
        }
        #[cfg(not(avx2))]
        fn replace_compose_vectorcall(dst: &mut Cube3, a: &Cube3, b: &Cube3) {
            inner(dst, a, b);
        }
        replace_compose_vectorcall(self, a, b);
    }

    #[inline(always)]
    fn replace_inverse(&mut self, a: &Self) {
        // Benchmarked on a 2x Intel Xeon E5-2667 v3 VM: 6.27ns
        fn inner(dst: &mut Cube3, a: &Cube3) {
            let perm = a.0 & PERM_MASK;
            let mut added_ori = a.0 ^ perm;

            let mut pow_3 = avx2_swizzle_lo(perm, perm);
            pow_3 = avx2_swizzle_lo(pow_3, perm);
            let mut inverse = avx2_swizzle_lo(pow_3, pow_3);
            inverse = avx2_swizzle_lo(inverse, inverse);
            inverse = avx2_swizzle_lo(avx2_swizzle_lo(inverse, inverse), pow_3);
            inverse = avx2_swizzle_lo(inverse, inverse);
            inverse = avx2_swizzle_lo(inverse, inverse);
            inverse = avx2_swizzle_lo(inverse, inverse);
            inverse = avx2_swizzle_lo(avx2_swizzle_lo(inverse, inverse), perm);
            inverse = avx2_swizzle_lo(inverse, inverse);
            inverse = avx2_swizzle_lo(inverse, inverse);
            inverse = avx2_swizzle_lo(inverse, inverse);
            inverse = avx2_swizzle_lo(inverse, inverse);
            inverse = avx2_swizzle_lo(avx2_swizzle_lo(inverse, inverse), pow_3);
            inverse = avx2_swizzle_lo(avx2_swizzle_lo(inverse, inverse), perm);

            added_ori += added_ori;
            added_ori = added_ori.simd_min(added_ori - ORI_CARRY_INVERSE);
            added_ori = avx2_swizzle_lo(added_ori, inverse);
            *dst = Cube3(inverse | added_ori);
        }
        #[cfg(avx2)]
        extern "vectorcall" fn replace_inverse_vectorcall(dst: &mut Cube3, a: &Cube3) {
            inner(dst, a)
        }
        #[cfg(not(avx2))]
        fn replace_inverse_vectorcall(dst: &mut Cube3, a: &Cube3) {
            inner(dst, a)
        }
        replace_inverse_vectorcall(self, a);
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[crate::phase2::puzzle::OrientedPartition],
        multi_bv: [u16; 2],
    ) -> bool {
        todo!();
    }
}

impl Cube3 {
    #[inline(always)]
    pub fn replace_inverse_brute(&mut self, a: &Self) {
        // Benchmarked on a 2x Intel Xeon E5-2667 v3 VM: 6.80ns
        fn inner(dst: &mut Cube3, a: &Cube3) {
            let perm = a.0 & PERM_MASK;
            let mut added_ori = a.0 ^ perm;

            let mut inverse = u8x32::from_array([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 13, 14, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 12, 13, 14, 15,
            ]);

            const IDENTITY: u8x32 = u8x32::from_array([
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
                10, 11, 12, 13, 14, 15,
            ]);
            macro_rules! brute_unroll {
                ($i:literal) => {
                    let inv_trial = u8x32::splat($i);
                    let inv_correct = IDENTITY
                        .simd_eq(avx2_swizzle_lo(perm, inv_trial))
                        .to_int()
                        .cast();
                    inverse = (inv_trial & inv_correct) | inverse;
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

            added_ori += added_ori;
            added_ori = added_ori.simd_min(added_ori - ORI_CARRY_INVERSE);
            added_ori = avx2_swizzle_lo(added_ori, inverse);
            *dst = Cube3(inverse | added_ori);
        }
        #[cfg(avx2)]
        extern "vectorcall" fn replace_inverse_brute_vectorcall(dst: &mut Cube3, a: &Cube3) {
            inner(dst, a)
        }
        #[cfg(not(avx2))]
        fn replace_inverse_brute_vectorcall(dst: &mut Cube3, a: &Cube3) {
            inner(dst, a)
        }
        replace_inverse_brute_vectorcall(self, a);
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;
    use crate::phase2::puzzle::{tests::apply_moves, PuzzleDef};
    use puzzle_geometry::ksolve::KPUZZLE_3X3;

    #[test]
    #[cfg_attr(not(avx2), ignore)]
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

    #[bench]
    #[cfg_attr(not(avx2), ignore)]
    fn bench_brute_force_inversion(b: &mut test::Bencher) {
        let cube3_def: PuzzleDef<Cube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let mut result = solved.clone();
        let order_1260 = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 100);
        b.iter(|| {
            test::black_box(&mut result).replace_inverse_brute(test::black_box(&order_1260));
        });
    }
}
