#![cfg_attr(any(avx2, not(simd8and16)), allow(dead_code, unused_variables))]

use super::common::Cube3Interface;
use crate::phase2::puzzle::OrientedPartition;
use std::{
    fmt,
    hash::Hash,
    num::NonZeroU8,
    simd::{
        cmp::{SimdOrd, SimdPartialEq, SimdPartialOrd},
        num::SimdInt,
        u8x8, u8x16,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct Cube3 {
    pub ep: u8x16,
    pub eo: u8x16,
    pub cp: u8x8,
    pub co: u8x8,
}

const CO_INV_SWIZZLE: u8x8 = u8x8::from_array([0, 2, 1, 0, 2, 1, 0, 0]);
const EP_IDENTITY: u8x16 =
    u8x16::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
const CP_IDENTITY: u8x8 = u8x8::from_array([0, 1, 2, 3, 4, 5, 6, 7]);
const EDGE_ORI_MASK: u8x16 = u8x16::splat(0b0001_0000);
const EDGE_PERM_MASK: u8x16 = u8x16::splat(0b0000_1111);
const CORNER_ORI_MASK: u8x8 = u8x8::splat(0b0011_0000);
const CORNER_PERM_MASK: u8x8 = u8x8::splat(0b0000_0111);
const CORNER_ORI_CARRY: u8x8 = u8x8::splat(0x30);

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
        // Benchmarked on a 2025 Mac M4: 1.67ns
        self.ep = a.ep.swizzle_dyn(b.ep);
        self.eo = (a.eo.swizzle_dyn(b.ep) + b.eo) & u8x16::splat(1);
        self.cp = a.cp.swizzle_dyn(b.cp);
        const CO_MOD_SWIZZLE: u8x8 = u8x8::from_array([0, 1, 2, 0, 1, 2, 0, 0]);
        self.co = CO_MOD_SWIZZLE.swizzle_dyn(a.co.swizzle_dyn(b.cp) + b.co);
    }

    fn replace_inverse(&mut self, a: &Self) {
        // Permutation inversion taken from Andrew Skalski's vcube[1]. The
        // addition sequence was generated using [2].
        // [1] https://github.com/Voltara/vcube
        // [2] http://wwwhomes.uni-bielefeld.de/achim/addition_chain.html
        //
        // Benchmarked on a 2025 Mac M4: 2.5ns
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
        self.eo = a.eo.swizzle_dyn(self.ep);

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
        self.co = CO_INV_SWIZZLE.swizzle_dyn(a.co).swizzle_dyn(self.cp);
    }

    fn induces_sorted_cycle_type(&self, sorted_cycle_type: &[OrientedPartition; 2]) -> bool {
        // Benchmarked on a 2025 Mac M4: 14.34ns (worst case) 4.04ns (average)

        // Helps avoid bounds checks in codegen
        #![allow(clippy::int_plus_one)]

        let mut seen = self.cp.simd_eq(CP_IDENTITY);
        let oriented_one_cycle_corner_mask = seen & self.co.simd_ne(u8x8::splat(0));
        let mut cycle_type_pointer =
            oriented_one_cycle_corner_mask.to_bitmask().count_ones() as usize;
        // Check oriented one cycles
        if cycle_type_pointer != 0
            && (cycle_type_pointer - 1 >= sorted_cycle_type[0].len()
                || sorted_cycle_type[0][cycle_type_pointer - 1] != (1.try_into().unwrap(), true))
        {
            return false;
        }

        let mut i = NonZeroU8::new(2).unwrap();
        let mut iter_cp = self.cp;
        let mut iter_co = self.co;
        while !seen.all() {
            iter_cp = iter_cp.swizzle_dyn(self.cp);
            iter_co = iter_co.swizzle_dyn(self.cp) + self.co;

            let identity_eq = iter_cp.simd_eq(CP_IDENTITY);
            let new_corners = identity_eq & !seen;
            seen |= identity_eq;

            let i_corner_cycle_count = new_corners.to_bitmask().count_ones();
            if i_corner_cycle_count > 0 {
                // x % 3 == 0 fast, https://lomont.org/posts/2017/divisibility-testing/
                // for some reason the compiler wasn't doing this optimization,
                // see https://github.com/rust-lang/portable-simd/issues/453
                let mut oriented_corner_mask =
                    (iter_co * u8x8::splat(171)).simd_gt(u8x8::splat(85));
                oriented_corner_mask &= new_corners;
                let i_oriented_corner_cycle_count = oriented_corner_mask.to_bitmask().count_ones();

                // Unoriented cycles
                if i_oriented_corner_cycle_count != i_corner_cycle_count {
                    cycle_type_pointer += ((i_corner_cycle_count - i_oriented_corner_cycle_count)
                        / i.get() as u32) as usize;
                    if cycle_type_pointer - 1 >= sorted_cycle_type[0].len()
                        || sorted_cycle_type[0][cycle_type_pointer - 1] != (i, false)
                    {
                        return false;
                    }
                }

                // Oriented cycles
                if i_oriented_corner_cycle_count != 0 {
                    cycle_type_pointer += (i_oriented_corner_cycle_count / i.get() as u32) as usize;
                    if cycle_type_pointer - 1 >= sorted_cycle_type[0].len()
                        || sorted_cycle_type[0][cycle_type_pointer - 1] != (i, true)
                    {
                        return false;
                    }
                }
            }
            // SAFETY: this loop will only ever run 8 times at max because that
            // is the longest cycle length among corners
            i = unsafe { NonZeroU8::new_unchecked(i.get() + 1) };
        }

        if cycle_type_pointer != sorted_cycle_type[0].len() {
            return false;
        }

        let mut seen = self.ep.simd_eq(EP_IDENTITY);
        let oriented_one_cycle_edge_mask = seen & self.eo.simd_ne(u8x16::splat(0));
        cycle_type_pointer = oriented_one_cycle_edge_mask.to_bitmask().count_ones() as usize;
        // Check oriented one cycles
        if cycle_type_pointer != 0
            && (cycle_type_pointer - 1 >= sorted_cycle_type[1].len()
                || sorted_cycle_type[1][cycle_type_pointer - 1] != (1.try_into().unwrap(), true))
        {
            return false;
        }

        i = NonZeroU8::new(2).unwrap();
        let mut iter_ep = self.ep;
        let mut iter_eo = self.eo;
        while !seen.all() {
            iter_ep = iter_ep.swizzle_dyn(self.ep);
            iter_eo = iter_eo.swizzle_dyn(self.ep) + self.eo;

            let identity_eq = iter_ep.simd_eq(EP_IDENTITY);
            let new_edges = identity_eq & !seen;
            seen |= identity_eq;

            let i_edge_cycle_count = new_edges.to_bitmask().count_ones();
            if i_edge_cycle_count > 0 {
                let mut oriented_edge_mask = (iter_eo & u8x16::splat(1)).simd_ne(u8x16::splat(0));
                oriented_edge_mask &= new_edges;
                let i_oriented_edge_cycle_count = oriented_edge_mask.to_bitmask().count_ones();

                // Unoriented cycles
                if i_oriented_edge_cycle_count != i_edge_cycle_count {
                    cycle_type_pointer += ((i_edge_cycle_count - i_oriented_edge_cycle_count)
                        / i.get() as u32) as usize;
                    if cycle_type_pointer - 1 >= sorted_cycle_type[1].len()
                        || sorted_cycle_type[1][cycle_type_pointer - 1] != (i, false)
                    {
                        return false;
                    }
                }

                // Oriented cycles
                if i_oriented_edge_cycle_count != 0 {
                    cycle_type_pointer += (i_oriented_edge_cycle_count / i.get() as u32) as usize;
                    if cycle_type_pointer - 1 >= sorted_cycle_type[1].len()
                        || sorted_cycle_type[1][cycle_type_pointer - 1] != (i, true)
                    {
                        return false;
                    }
                }
            }
            // SAFETY: this loop will only ever run 12 times at max because that
            // is the longest cycle length among edges
            i = unsafe { NonZeroU8::new_unchecked(i.get() + 1) };
        }

        cycle_type_pointer == sorted_cycle_type[1].len()
    }

    fn orbit_bytes(&self, orbit_index: usize) -> (&[u8], &[u8]) {
        todo!()
    }

    fn exact_hash_orbit(&self, orbit_index: usize) -> u64 {
        todo!()
    }

    fn approximate_hash_orbit(&self, orbit_index: usize) -> impl Hash {
        todo!()
    }
}

#[derive(PartialEq, Clone)]
// TODO
pub struct CompressedCube3 {
    edges: u8x16,
    corners: u8x8,
}

impl fmt::Debug for CompressedCube3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ep = [0; 16];
        let mut eo = [0; 16];
        let mut cp = [0; 8];
        let mut co = [0; 8];

        for i in 0..16 {
            ep[i] = self.edges[i] & 0b1111;
            eo[i] = self.edges[i] >> 4;
        }

        for i in 0..8 {
            cp[i] = self.corners[i] & 0b111;
            co[i] = self.corners[i] >> 4;
        }

        f.debug_struct("CompressedCube3")
            .field("ep", &ep)
            .field("eo", &eo)
            .field("cp", &cp)
            .field("co", &co)
            .finish()
    }
}

impl Cube3Interface for CompressedCube3 {
    fn from_sorted_transformations(sorted_transformations: &[Vec<(u8, u8)>]) -> Self {
        let corners_transformation = &sorted_transformations[0];
        let edges_transformation = &sorted_transformations[1];

        let mut edges = u8x16::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 13, 14, 15]);
        let mut corners = u8x8::splat(0);

        for i in 0..12 {
            let (perm, orientation_delta) = edges_transformation[i];
            edges[i] = perm | (orientation_delta << 4);
        }

        for i in 0..8 {
            let (perm, orientation) = corners_transformation[i];
            corners[i] = perm | (orientation << 4);
        }

        CompressedCube3 { edges, corners }
    }

    fn replace_compose(&mut self, a: &Self, b: &Self) {
        // Benchmarked on a 2025 Mac M4: 1.12ns
        let mut edges_composed = a.edges.swizzle_dyn(b.edges & EDGE_PERM_MASK);
        edges_composed ^= b.edges & EDGE_ORI_MASK;

        let mut corners_composed = a.corners.swizzle_dyn(b.corners & CORNER_PERM_MASK);
        corners_composed += b.corners & CORNER_ORI_MASK;
        corners_composed = corners_composed.simd_min(corners_composed - CORNER_ORI_CARRY);

        self.edges = edges_composed;
        self.corners = corners_composed;
    }

    fn replace_inverse(&mut self, a: &Self) {
        todo!()
    }

    fn induces_sorted_cycle_type(&self, sorted_cycle_type: &[OrientedPartition; 2]) -> bool {
        todo!()
    }

    fn orbit_bytes(&self, orbit_index: usize) -> (&[u8], &[u8]) {
        todo!()
    }

    fn exact_hash_orbit(&self, orbit_index: usize) -> u64 {
        todo!()
    }

    fn approximate_hash_orbit(&self, orbit_index: usize) -> impl Hash {
        todo!()
    }
}

impl Cube3 {
    pub fn replace_inverse_brute(&mut self, a: &Self) {
        // Benchmarked on a 2025 Mac M4: 4.11ns
        self.ep = u8x16::from_array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 13, 14, 15]);
        self.cp = u8x8::splat(0);
        // Brute force the inverse by checking all possible values and
        // using a mask to check when equal to identity (also inspired by
        // Andrew Skalski's vcube).
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

        self.eo = a.eo.swizzle_dyn(self.ep);
        self.co = CO_INV_SWIZZLE.swizzle_dyn(a.co).swizzle_dyn(self.cp);
    }

    pub fn replace_inverse_raw(&mut self, a: &Self) {
        // Benchmarked on a 2025 Mac M4: 3.8ns

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

        self.eo = a.eo.swizzle_dyn(self.ep);
        self.co = CO_INV_SWIZZLE.swizzle_dyn(a.co).swizzle_dyn(self.cp);
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;
    use crate::phase2::puzzle::{PuzzleDef, apply_moves};
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
            let mut result_1 = solved.clone();
            let mut result_2 = solved.clone();
            for _ in 0..20 {
                let move_index = fastrand::choice(0_u8..18).unwrap();
                let move_ = &cube3_def.moves[move_index as usize];
                result_1.replace_compose(&result_2, &move_.puzzle_state);
                std::mem::swap(&mut result_2, &mut result_1);
            }
            result_1.replace_inverse_brute(&result_2);
            result_2.replace_compose(&result_1, &result_2.clone());
            assert_eq!(result_2, solved);
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
            let mut result_1 = solved.clone();
            let mut result_2 = solved.clone();
            for _ in 0..20 {
                let move_index = fastrand::choice(0_u8..18).unwrap();
                let move_ = &cube3_def.moves[move_index as usize];
                result_1.replace_compose(&result_2, &move_.puzzle_state);
                std::mem::swap(&mut result_2, &mut result_1);
            }
            result_1.replace_inverse_raw(&result_2);
            result_2.replace_compose(&result_1, &result_2.clone());
            assert_eq!(result_2, solved);
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
