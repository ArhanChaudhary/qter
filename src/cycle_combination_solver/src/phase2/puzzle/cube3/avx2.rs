#![cfg_attr(not(avx2), allow(dead_code, unused_variables))]

use crate::phase2::puzzle::OrientedPartition;

use super::common::Cube3Interface;
#[cfg(all(avx2, target_arch = "x86"))]
use core::arch::x86::_mm256_shuffle_epi8;
#[cfg(all(avx2, target_arch = "x86_64"))]
use core::arch::x86_64::_mm256_shuffle_epi8;
use std::{
    fmt,
    hash::{Hash, Hasher},
    num::NonZeroU8,
    simd::{
        cmp::{SimdOrd, SimdPartialEq, SimdPartialOrd},
        num::SimdInt,
        u8x16, u8x32,
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

const PERM_MASK: u8x32 = u8x32::splat(0b0000_1111);
const ORI_MASK: u8x32 = u8x32::splat(0b0011_0000);
const ORI_CARRY_INVERSE: u8x32 = u8x32::from_array([
    0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10,
    0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30,
]);
const IDENTITY: u8x32 = u8x32::from_array([
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
]);
const EDGE_START: usize = 0;
const CORNER_START: usize = 16;

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
        // Benchmarked on a 2x Intel Xeon E5-2667 v3: 1.55ns
        fn inner(dst: &mut Cube3, a: &Cube3, b: &Cube3) {
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
        // Benchmarked on a 2x Intel Xeon E5-2667 v3: 6.27ns
        fn inner(dst: &mut Cube3, a: &Cube3) {
            let perm = a.0 & PERM_MASK;

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

            let mut added_ori = a.0 & ORI_MASK;
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

    fn induces_sorted_cycle_type(&self, sorted_cycle_type: &[OrientedPartition; 2]) -> bool {
        // Benchmarked on a 2x Intel Xeon E5-2667 v3: 39.94ns (worst) 12.71ns (average)
        // see simd8and16 for explanation
        #![allow(clippy::int_plus_one)]

        let compose_ori = self.0 & ORI_MASK;
        let mut seen = (self.0 & PERM_MASK).simd_eq(IDENTITY);

        // TODO: extract leads to extra vextracti128 instructions, could trying
        // to remove them be faster?
        let oriented_one_cycle_corner_mask = seen.extract::<CORNER_START, 16>()
            & compose_ori
                .extract::<CORNER_START, 16>()
                .simd_ne(u8x16::splat(0));
        let mut corner_cycle_type_pointer =
            oriented_one_cycle_corner_mask.to_bitmask().count_ones() as usize;
        // Check oriented one cycles
        if corner_cycle_type_pointer != 0
            && (corner_cycle_type_pointer - 1 >= sorted_cycle_type[0].len()
                || sorted_cycle_type[0][corner_cycle_type_pointer - 1]
                    != (1.try_into().unwrap(), true))
        {
            return false;
        }

        let oriented_one_cycle_edge_mask = seen.extract::<EDGE_START, 16>()
            & compose_ori
                .extract::<EDGE_START, 16>()
                .simd_ne(u8x16::splat(0));
        let mut edge_cycle_type_pointer =
            oriented_one_cycle_edge_mask.to_bitmask().count_ones() as usize;
        // Check oriented one cycles
        if edge_cycle_type_pointer != 0
            && (edge_cycle_type_pointer - 1 >= sorted_cycle_type[1].len()
                || sorted_cycle_type[1][edge_cycle_type_pointer - 1]
                    != (1.try_into().unwrap(), true))
        {
            return false;
        }

        let mut i = NonZeroU8::new(2).unwrap();
        let mut iter = self.0;
        while !seen.all() {
            iter = avx2_swizzle_lo(iter, self.0) + compose_ori;

            let identity_eq = (iter & PERM_MASK).simd_eq(IDENTITY);
            let new_pieces = identity_eq & !seen;
            seen |= identity_eq;

            let new_corners = new_pieces.extract::<CORNER_START, 16>();
            let i_corner_cycle_count = new_corners.to_bitmask().count_ones();
            if i_corner_cycle_count > 0 {
                let iter_co_mod =
                    (iter.extract::<CORNER_START, 16>() >> u8x16::splat(4)) * u8x16::splat(171);
                let oriented_corner_mask = new_corners & iter_co_mod.simd_gt(u8x16::splat(85));
                let i_oriented_corner_cycle_count = oriented_corner_mask.to_bitmask().count_ones();

                // Unoriented cycles
                if i_oriented_corner_cycle_count != i_corner_cycle_count {
                    corner_cycle_type_pointer += ((i_corner_cycle_count
                        - i_oriented_corner_cycle_count)
                        / i.get() as u32) as usize;
                    if corner_cycle_type_pointer - 1 >= sorted_cycle_type[0].len()
                        || sorted_cycle_type[0][corner_cycle_type_pointer - 1] != (i, false)
                    {
                        return false;
                    }
                }

                // Oriented cycles
                if i_oriented_corner_cycle_count != 0 {
                    corner_cycle_type_pointer +=
                        (i_oriented_corner_cycle_count / i.get() as u32) as usize;
                    if corner_cycle_type_pointer - 1 >= sorted_cycle_type[0].len()
                        || sorted_cycle_type[0][corner_cycle_type_pointer - 1] != (i, true)
                    {
                        return false;
                    }
                }
            }

            let new_edges = new_pieces.extract::<EDGE_START, 16>();
            let i_edge_cycle_count = new_edges.to_bitmask().count_ones();
            if i_edge_cycle_count > 0 {
                let iter_eo_mod = iter.extract::<EDGE_START, 16>() & u8x16::splat(0b0001_0000);
                let oriented_edge_mask = new_edges & iter_eo_mod.simd_ne(u8x16::splat(0));
                let i_oriented_edge_cycle_count = oriented_edge_mask.to_bitmask().count_ones();

                // Unoriented cycles
                if i_oriented_edge_cycle_count != i_edge_cycle_count {
                    edge_cycle_type_pointer += ((i_edge_cycle_count - i_oriented_edge_cycle_count)
                        / i.get() as u32) as usize;
                    if edge_cycle_type_pointer - 1 >= sorted_cycle_type[1].len()
                        || sorted_cycle_type[1][edge_cycle_type_pointer - 1] != (i, false)
                    {
                        return false;
                    }
                }

                // Oriented cycles
                if i_oriented_edge_cycle_count != 0 {
                    edge_cycle_type_pointer +=
                        (i_oriented_edge_cycle_count / i.get() as u32) as usize;
                    if edge_cycle_type_pointer - 1 >= sorted_cycle_type[1].len()
                        || sorted_cycle_type[1][edge_cycle_type_pointer - 1] != (i, true)
                    {
                        return false;
                    }
                }
            }
            // SAFETY: this loop will only ever run 12 times at max because that
            // is the longest cycle length among edges
            i = unsafe { i.unchecked_add(1) };
        }

        corner_cycle_type_pointer == sorted_cycle_type[0].len()
            && edge_cycle_type_pointer == sorted_cycle_type[1].len()
    }
}

impl Cube3 {
    #[inline(always)]
    pub fn replace_inverse_brute(&mut self, a: &Self) {
        // Benchmarked on a 2x Intel Xeon E5-2667 v3: 6.77ns
        fn inner(dst: &mut Cube3, a: &Cube3) {
            let perm = a.0 & PERM_MASK;

            let mut inverse = u8x32::from_array([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 13, 14, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 12, 13, 14, 15,
            ]);

            macro_rules! brute_unroll {
                ($i:literal) => {
                    let inv_trial = u8x32::splat($i);
                    let inv_correct = IDENTITY
                        .simd_eq(avx2_swizzle_lo(perm, inv_trial))
                        .to_int()
                        .cast();
                    inverse |= inv_trial & inv_correct;
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

            let mut added_ori = a.0 & ORI_MASK;
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
