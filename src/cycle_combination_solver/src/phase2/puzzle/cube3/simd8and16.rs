//! A SIMD optimized implementation for 3x3 cubes for platforms that support
//! 8 and 16 byte SIMD.

#![cfg_attr(any(avx2, not(simd8and16)), allow(dead_code, unused_variables))]

use super::common::{CUBE_3_SORTED_ORBIT_DEFS, Cube3Interface};
use crate::phase2::{FACT_UNTIL_19, puzzle::OrientedPartition};
use std::{
    fmt,
    hash::Hash,
    num::NonZeroU8,
    simd::{
        LaneCount, Simd, SupportedLaneCount,
        cmp::{SimdOrd, SimdPartialEq, SimdPartialOrd},
        num::{SimdInt, SimdUint},
        u8x8, u8x16,
    },
};

/// An uncompressed 3x3 cube representation. This is a combination of
/// (edge permutation, edge orientation, corner permutation, corner orientation)
/// which uniquely identifies any cube state
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct UncompressedCube3 {
    pub ep: u8x16,
    pub eo: u8x16,
    pub cp: u8x8,
    pub co: u8x8,
}

/// A lookup table used to correct orientation during composition
const CO_MOD_SWIZZLE: u8x8 = u8x8::from_array([0, 1, 2, 0, 1, 0, 0, 0]);
/// A lookup table used to inverse a corner orientation.
const CO_INV_SWIZZLE: u8x8 = u8x8::from_array([0, 2, 1, 0, 0, 0, 0, 0]);
/// The identity permutation for edges.
const EP_IDENTITY: u8x16 =
    u8x16::from_array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
/// The identity permutation for corners.
const CP_IDENTITY: u8x8 = u8x8::from_array([0, 1, 2, 3, 4, 5, 6, 7]);

/// Masks for edge and corner orientations and permutations.
const EDGE_ORI_MASK: u8x16 = u8x16::splat(0b0001_0000);
const EDGE_PERM_MASK: u8x16 = u8x16::splat(0b0000_1111);
const CORNER_ORI_MASK: u8x8 = u8x8::splat(0b0011_0000);
const CORNER_PERM_MASK: u8x8 = u8x8::splat(0b0000_0111);

/// Experimental carry for corner orientations, not used
#[allow(dead_code)]
const CORNER_ORI_CARRY: u8x8 = u8x8::splat(3);

/// Corner or edge data for `UncompressedCube3` orbits
#[derive(Hash)]
pub enum UncompressedCube3Orbit {
    /// (cp, co)
    Corners((u8x8, u8x8)),
    /// (ep, eo)
    Edges((u8x16, u8x16)),
}

// TODO: move this to orbit_puzzle/simd8and16.rs
/// Efficently exactly hash an orbit into a u64, panicking at compile-time if
/// not possible. This function uses a combination of SIMD lehmer coding and an
/// efficient n-ary base hash. Uses `u16`s for const generics because usize
/// implements From<u16>.
fn exact_hasher_orbit<const PIECE_COUNT: u16, const ORI_COUNT: u16, const LEN: usize>(
    perm: Simd<u8, LEN>,
    ori: Simd<u8, LEN>,
) -> u64
where
    LaneCount<LEN>: SupportedLaneCount,
{
    // Powers of ORI_COUNT used to efficiently hash the orientation to an n-ary
    // base. The hash is essentially a dot product of the orientation vector
    // with the powers of ORI_COUNT
    let powers: Simd<u16, LEN> = const {
        // Everything not a power must be zero to make sure nothing interferes
        let mut arr = [0; LEN];
        let mut i = 0;
        // We do an important check that the next power does not overflow `u16`.
        // The dot product will eventually be collapsed to a value larger than
        // ORI_COUNT.pow(PIECE_COUNT - 2) but less than
        // ORI_COUNT.pow(PIECE_COUNT - 1).
        u16::checked_pow(ORI_COUNT, PIECE_COUNT as u32 - 1).unwrap();
        // The sum of the orientation vector must be divisible by ORI_COUNT.
        // As a consequence, you don't need the last element to uniquely
        // identify an orientation vector, so we skip processing for it by
        // only computing powers up to PIECE_COUNT - 1
        while i < PIECE_COUNT - 1 {
            // Under the hood LLVM splits up the dot product calculation into
            // chunks of 128 bit registers so having a the smallest possible
            // data type (u16) is important
            arr[i as usize] = u16::checked_pow(
                ORI_COUNT,
                (
                    // The powers are computed in reverse order to match the
                    // order of lexicographic permutation with replacement.
                    // Reverse order in general is len - i - 1, and len is
                    // PIECE_COUNT - 1
                    (PIECE_COUNT - 1) - i - 1
                ) as u32,
            )
            .unwrap();
            i += 1;
        }
        Simd::<u16, LEN>::from_array(arr)
    };
    // We compute: lehmer code * number_of_states(n-ary hash) + n-ary hash
    //
    // One thing to note about the last element for Lehmer codes is no matter
    // what, there will always be an equal number of elements to its left that
    // are less than it. This allows us to hard code it to 0 and iterate from 0
    // to PIECE_COUNT - 1
    (0..usize::from(PIECE_COUNT) - 1)
        .map(|i| {
            let lt_before_current_count = if i == 0 {
                // There are no elements left of the first element less than it
                u64::from(perm[0])
            } else {
                // Count how many elements to the left of the current element
                // are less than it
                let lt_current_mask = perm.simd_lt(Simd::<u8, LEN>::splat(perm[i]));
                u64::from((lt_current_mask.to_bitmask() >> i).count_ones())
            };
            // FACT_UNTIL_19[i] = i!
            let fact = FACT_UNTIL_19[usize::from(PIECE_COUNT) - 1 - i];
            lt_before_current_count * fact
        })
        .sum::<u64>()
        // Orientation is a permutation with replacement. The number of states
        // is trivially ORI_COUNT.pow(PIECE_COUNT), but subtract one because the
        // last element is ignored as described above
        * u64::from(ORI_COUNT.pow(u32::from(PIECE_COUNT) - 1))
        // Compute the aforementioned dot product
        + u64::from((ori.cast::<u16>() * powers).reduce_sum())
}

impl Cube3Interface for UncompressedCube3 {
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

        UncompressedCube3 { ep, eo, cp, co }
    }

    fn replace_compose(&mut self, a: &Self, b: &Self) {
        // Benchmarked on a 2025 Mac M4: 1.67ns

        // Compose edge permutation using the built-in SIMD swizzle
        self.ep = a.ep.swizzle_dyn(b.ep);
        // "The Cubie Level" of Kociemba's [website] explains that orientation
        // during composition changes like so: (A*B)(x).o=A(B(x).c).o+B(x).o
        // Edge orientation is defined as either 0 or 1. Adding two orientations
        // together may result in 2, so we need to modulo 2 the result
        self.eo = (a.eo.swizzle_dyn(b.ep) + b.eo) & u8x16::splat(1);
        // Compose corner permutation using the built-in SIMD swizzle
        self.cp = a.cp.swizzle_dyn(b.cp);
        // Like the edge orientation, corner orientation is defined as
        // either 0, 1, or 2. Adding two corner orientations together may result
        // in 3 or 4. It was found fastest to use a lookup table to perform
        // this correction
        self.co = CO_MOD_SWIZZLE.swizzle_dyn(a.co.swizzle_dyn(b.cp) + b.co);
    }

    fn replace_inverse(&mut self, a: &Self) {
        // Benchmarked on a 2025 Mac M4: 2.5ns
        //
        // See `replace_inverse` in avx2.rs for explanation. Note that there
        // does not seem to be any speed difference when these instructions are
        // reordered (codegen puts all u8x8 and u8x16 swizzles together)
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
        // eo doesn't change during inversion; all we need to do is permute it
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
        // slightly slower ...
        // let mut added_ori = a.co + a.co;
        // added_ori = added_ori.simd_min(added_ori - CORNER_ORI_MASK);
        // self.co = added_ori.swizzle_dyn(self.cp);
        self.co = CO_INV_SWIZZLE.swizzle_dyn(a.co).swizzle_dyn(self.cp);
    }

    fn induces_sorted_cycle_type(&self, sorted_cycle_type: &[OrientedPartition; 2]) -> bool {
        // Benchmarked on a 2025 Mac M4: TODO (worst case) TODO (average)
        //
        // Explanation in `induces_sorted_cycle_type` in avx2.rs

        let mut seen_cp = self.cp.simd_eq(CP_IDENTITY);
        let oriented_one_cycle_corner_mask = seen_cp & self.co.simd_ne(u8x8::splat(0));
        let mut cycle_type_pointer =
            (oriented_one_cycle_corner_mask.to_bitmask().count_ones() as usize).wrapping_sub(1);
        // Check oriented one cycles
        if cycle_type_pointer == usize::MAX {
            if let Some(&first_cycle) = sorted_cycle_type[0].first() {
                if first_cycle == (1.try_into().unwrap(), true) {
                    return false;
                }
            }
        } else if cycle_type_pointer >= sorted_cycle_type[0].len()
            || sorted_cycle_type[0][cycle_type_pointer] != (1.try_into().unwrap(), true)
        {
            return false;
        }

        let mut reps = NonZeroU8::new(2).unwrap();
        let mut iter_cp = self.cp;
        let mut iter_co = self.co;
        while !seen_cp.all() {
            iter_cp = iter_cp.swizzle_dyn(self.cp);
            iter_co = iter_co.swizzle_dyn(self.cp) + self.co;

            let cp_identity_eq = iter_cp.simd_eq(CP_IDENTITY);
            let new_corners = cp_identity_eq & !seen_cp;
            seen_cp |= cp_identity_eq;

            // Moving this inside of the if statement adds instructions for some
            // reason; see https://www.diffchecker.com/graztmK5/
            let reps_corner_cycle_count = new_corners.to_bitmask().count_ones();
            if new_corners.any() {
                let mut oriented_corner_mask =
                    (iter_co * u8x8::splat(171)).simd_gt(u8x8::splat(85));
                oriented_corner_mask &= new_corners;
                let reps_oriented_corner_cycle_count =
                    oriented_corner_mask.to_bitmask().count_ones();

                // Unoriented cycles
                if reps_oriented_corner_cycle_count != reps_corner_cycle_count {
                    cycle_type_pointer = cycle_type_pointer.wrapping_add(
                        ((reps_corner_cycle_count - reps_oriented_corner_cycle_count)
                            / u32::from(reps.get())) as usize,
                    );
                    if cycle_type_pointer >= sorted_cycle_type[0].len()
                        || sorted_cycle_type[0][cycle_type_pointer] != (reps, false)
                    {
                        return false;
                    }
                }

                // Oriented cycles
                if reps_oriented_corner_cycle_count != 0 {
                    cycle_type_pointer = cycle_type_pointer.wrapping_add(
                        (reps_oriented_corner_cycle_count / u32::from(reps.get())) as usize,
                    );
                    if cycle_type_pointer >= sorted_cycle_type[0].len()
                        || sorted_cycle_type[0][cycle_type_pointer] != (reps, true)
                    {
                        return false;
                    }
                }
            }
            // SAFETY: this loop will only ever run 8 times at max because that
            // is the longest cycle length among corners
            reps = unsafe { NonZeroU8::new_unchecked(reps.get() + 1) };
        }

        if cycle_type_pointer != sorted_cycle_type[0].len().wrapping_sub(1) {
            return false;
        }

        let mut seen_ep = self.ep.simd_eq(EP_IDENTITY);
        let oriented_one_cycle_edge_mask = seen_ep & self.eo.simd_ne(u8x16::splat(0));
        cycle_type_pointer =
            (oriented_one_cycle_edge_mask.to_bitmask().count_ones() as usize).wrapping_sub(1);
        // Check oriented one cycles
        if cycle_type_pointer == usize::MAX {
            if let Some(&first_cycle) = sorted_cycle_type[1].first() {
                if first_cycle == (1.try_into().unwrap(), true) {
                    return false;
                }
            }
        } else if cycle_type_pointer >= sorted_cycle_type[1].len()
            || sorted_cycle_type[1][cycle_type_pointer] != (1.try_into().unwrap(), true)
        {
            return false;
        }

        reps = NonZeroU8::new(2).unwrap();
        let mut iter_ep = self.ep;
        let mut iter_eo = self.eo;
        while !seen_ep.all() {
            iter_ep = iter_ep.swizzle_dyn(self.ep);
            iter_eo = iter_eo.swizzle_dyn(self.ep) + self.eo;

            let ep_identity_eq = iter_ep.simd_eq(EP_IDENTITY);
            let new_edges = ep_identity_eq & !seen_ep;
            seen_ep |= ep_identity_eq;

            // Moving this inside of the if statement adds instructions
            let reps_edge_cycle_count = new_edges.to_bitmask().count_ones();
            if new_edges.any() {
                let mut oriented_edge_mask = (iter_eo & u8x16::splat(1)).simd_ne(u8x16::splat(0));
                oriented_edge_mask &= new_edges;
                let reps_oriented_edge_cycle_count = oriented_edge_mask.to_bitmask().count_ones();

                // Unoriented cycles
                if reps_oriented_edge_cycle_count != reps_edge_cycle_count {
                    cycle_type_pointer = cycle_type_pointer.wrapping_add(
                        ((reps_edge_cycle_count - reps_oriented_edge_cycle_count)
                            / u32::from(reps.get())) as usize,
                    );
                    if cycle_type_pointer >= sorted_cycle_type[1].len()
                        || sorted_cycle_type[1][cycle_type_pointer] != (reps, false)
                    {
                        return false;
                    }
                }

                // Oriented cycles
                if reps_oriented_edge_cycle_count != 0 {
                    cycle_type_pointer = cycle_type_pointer.wrapping_add(
                        (reps_oriented_edge_cycle_count / u32::from(reps.get())) as usize,
                    );
                    if cycle_type_pointer >= sorted_cycle_type[1].len()
                        || sorted_cycle_type[1][cycle_type_pointer] != (reps, true)
                    {
                        return false;
                    }
                }
            }
            // SAFETY: this loop will only ever run 12 times at max because that
            // is the longest cycle length among edges
            reps = unsafe { NonZeroU8::new_unchecked(reps.get() + 1) };
        }

        cycle_type_pointer == sorted_cycle_type[1].len().wrapping_sub(1)
    }

    fn orbit_bytes(&self, orbit_index: usize) -> ([u8; 16], [u8; 16]) {
        match orbit_index {
            0 => {
                let mut perm = [0; 16];
                let mut ori = [0; 16];
                self.cp.copy_to_slice(&mut perm);
                self.co.copy_to_slice(&mut ori);
                (perm, ori)
            }
            1 => (self.ep.to_array(), self.eo.to_array()),
            _ => panic!("Invalid orbit index"),
        }
    }

    fn exact_hasher_orbit(&self, orbit_index: usize) -> u64 {
        match orbit_index {
            0 => {
                const PIECE_COUNT: u16 = CUBE_3_SORTED_ORBIT_DEFS[0].piece_count.get() as u16;
                const ORI_COUNT: u16 = CUBE_3_SORTED_ORBIT_DEFS[0].orientation_count.get() as u16;

                exact_hasher_orbit::<
                    PIECE_COUNT,
                    ORI_COUNT,
                    { PIECE_COUNT.next_power_of_two() as usize },
                >(self.cp, self.co)
            }
            1 => {
                const PIECE_COUNT: u16 = CUBE_3_SORTED_ORBIT_DEFS[1].piece_count.get() as u16;
                const ORI_COUNT: u16 = CUBE_3_SORTED_ORBIT_DEFS[1].orientation_count.get() as u16;

                exact_hasher_orbit::<
                    PIECE_COUNT,
                    ORI_COUNT,
                    { PIECE_COUNT.next_power_of_two() as usize },
                >(self.ep, self.eo)
            }
            _ => panic!("Invalid orbit index"),
        }
    }

    #[allow(refining_impl_trait_reachable)]
    fn approximate_hash_orbit(&self, orbit_index: usize) -> UncompressedCube3Orbit {
        // TODO: using an enum works, but is this slow? same with compressedcube3
        match orbit_index {
            0 => UncompressedCube3Orbit::Corners((self.cp, self.co)),
            1 => UncompressedCube3Orbit::Edges((self.ep, self.eo)),
            _ => panic!("Invalid orbit index"),
        }
    }
}

/// A compressed 3x3 cube representation. The byte layout is equivalent to
/// Cube3 in `avx2.rs`, but uses 8 bytes for corners intead of 16.
// TODO: shift the corner bits one right
#[derive(PartialEq, Clone)]
pub struct Cube3 {
    edges: u8x16,
    corners: u8x8,
}

impl fmt::Debug for Cube3 {
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

#[derive(Hash)]
pub enum Cube3Orbit {
    Edges(u8x16),
    Corners(u8x8),
}

impl Cube3Interface for Cube3 {
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

        Cube3 { edges, corners }
    }

    fn replace_compose(&mut self, a: &Self, b: &Self) {
        // Benchmarked on a 2025 Mac M4: 1.12ns
        let mut edges_composed = a.edges.swizzle_dyn(b.edges & EDGE_PERM_MASK);
        edges_composed ^= b.edges & EDGE_ORI_MASK;

        let mut corners_composed = a.corners.swizzle_dyn(b.corners & CORNER_PERM_MASK);
        corners_composed += b.corners & CORNER_ORI_MASK;
        corners_composed = corners_composed.simd_min(corners_composed - CORNER_ORI_MASK);

        self.edges = edges_composed;
        self.corners = corners_composed;
    }

    fn replace_inverse(&mut self, a: &Self) {
        // Benchmarked on a 2025 Mac M4: TODO

        let ep = a.edges & EDGE_PERM_MASK;
        let mut pow_3_ep = ep.swizzle_dyn(ep);
        pow_3_ep = pow_3_ep.swizzle_dyn(ep);
        let mut inverse_ep = pow_3_ep.swizzle_dyn(pow_3_ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep).swizzle_dyn(pow_3_ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep).swizzle_dyn(ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep).swizzle_dyn(pow_3_ep);
        inverse_ep = inverse_ep.swizzle_dyn(inverse_ep).swizzle_dyn(ep);
        let mut inverse_eo = a.edges & EDGE_ORI_MASK;
        inverse_eo = inverse_eo.swizzle_dyn(inverse_ep);
        self.edges = inverse_eo | inverse_ep;

        let cp = a.corners & CORNER_PERM_MASK;
        let mut pow_3_cp = cp.swizzle_dyn(cp);
        pow_3_cp = pow_3_cp.swizzle_dyn(cp);
        let mut inverse_cp = pow_3_cp.swizzle_dyn(pow_3_cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp).swizzle_dyn(pow_3_cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp).swizzle_dyn(cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp).swizzle_dyn(pow_3_cp);
        inverse_cp = inverse_cp.swizzle_dyn(inverse_cp).swizzle_dyn(cp);
        let mut inverse_co = a.corners >> 4;
        inverse_co = CO_INV_SWIZZLE
            .swizzle_dyn(inverse_co)
            .swizzle_dyn(inverse_cp);
        inverse_co <<= 4;
        // slightly slower ...
        // let mut inverse_co = a.corners & CORNER_ORI_MASK;
        // inverse_co += inverse_co;
        // inverse_co = inverse_co.simd_min(inverse_co - CORNER_ORI_CARRY);
        // inverse_co = inverse_co.swizzle_dyn(inverse_cp);
        self.corners = inverse_co | inverse_cp;
    }

    fn induces_sorted_cycle_type(&self, sorted_cycle_type: &[OrientedPartition; 2]) -> bool {
        todo!()
    }

    fn orbit_bytes(&self, orbit_index: usize) -> ([u8; 16], [u8; 16]) {
        match orbit_index {
            0 => {
                let perm = self.corners & CORNER_PERM_MASK;
                let ori = self.corners >> 4;
                let mut perm_arr = [0; 16];
                let mut ori_arr = [0; 16];
                perm.copy_to_slice(&mut perm_arr);
                ori.copy_to_slice(&mut ori_arr);
                (perm_arr, ori_arr)
            }
            1 => {
                let perm = self.edges & EDGE_PERM_MASK;
                let ori = self.edges >> 4;
                (perm.to_array(), ori.to_array())
            }
            _ => panic!("Invalid orbit index"),
        }
    }

    fn exact_hasher_orbit(&self, orbit_index: usize) -> u64 {
        todo!()
    }

    #[allow(refining_impl_trait_reachable)]
    fn approximate_hash_orbit(&self, orbit_index: usize) -> Cube3Orbit {
        match orbit_index {
            0 => Cube3Orbit::Corners(self.corners),
            1 => Cube3Orbit::Edges(self.edges),
            _ => panic!("Invalid orbit index"),
        }
    }
}

impl UncompressedCube3 {
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
        // slightly slower ...
        // let mut added_ori = a.co + a.co;
        // added_ori = added_ori.simd_min(added_ori - CORNER_ORI_MASK);
        // self.co = added_ori.swizzle_dyn(self.cp);
        self.co = CO_INV_SWIZZLE.swizzle_dyn(a.co).swizzle_dyn(self.cp);
    }

    pub fn replace_inverse_raw(&mut self, a: &Self) {
        // Benchmarked on a 2025 Mac M4: 3.8ns

        for i in 0..12 {
            // SAFETY: ep is length 12, so i is always in bounds
            unsafe {
                *self
                    .ep
                    .as_mut_array()
                    .get_unchecked_mut(a.ep[i as usize] as usize) = i;
            }
            if i < 8 {
                // SAFETY: cp is length 8, so i is always in bounds
                unsafe {
                    *self
                        .cp
                        .as_mut_array()
                        .get_unchecked_mut(a.cp[i as usize] as usize) = i;
                }
            }
        }

        self.eo = a.eo.swizzle_dyn(self.ep);
        // slightly slower ...
        // let mut added_ori = a.co + a.co;
        // added_ori = added_ori.simd_min(added_ori - CORNER_ORI_MASK);
        // self.co = added_ori.swizzle_dyn(self.cp);
        self.co = CO_INV_SWIZZLE.swizzle_dyn(a.co).swizzle_dyn(self.cp);
    }
}

impl Cube3 {
    pub fn replace_inverse_brute(&mut self, a: &Self) {
        todo!();
    }

    pub fn replace_inverse_raw(&mut self, a: &Self) {
        todo!();
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
    fn test_uncompressed_brute_force_inversion() {
        let cube3_def: PuzzleDef<UncompressedCube3> = (&*KPUZZLE_3X3).try_into().unwrap();
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
    fn test_uncompressed_raw_inversion() {
        let cube3_def: PuzzleDef<UncompressedCube3> = (&*KPUZZLE_3X3).try_into().unwrap();
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
    fn bench_uncompressed_brute_force_inversion(b: &mut test::Bencher) {
        let cube3_def: PuzzleDef<UncompressedCube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let mut result = solved.clone();
        let order_1260 = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 100);
        b.iter(|| {
            test::black_box(&mut result).replace_inverse_brute(test::black_box(&order_1260));
        });
    }

    #[bench]
    #[cfg_attr(not(simd8and16), ignore)]
    fn bench_uncompressed_raw_inversion(b: &mut test::Bencher) {
        let cube3_def: PuzzleDef<UncompressedCube3> = (&*KPUZZLE_3X3).try_into().unwrap();
        let solved = cube3_def.new_solved_state();
        let mut result = solved.clone();
        let order_1260 = apply_moves(&cube3_def, &solved, "R U2 D' B D'", 100);
        b.iter(|| {
            test::black_box(&mut result).replace_inverse_raw(test::black_box(&order_1260));
        });
    }
}
