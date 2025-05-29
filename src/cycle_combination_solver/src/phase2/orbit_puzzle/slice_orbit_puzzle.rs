//! The default and fallback implementation for 3x3 orbits during pruning table
//! generation.

use super::{OrbitPuzzleConstructors, OrbitPuzzleState};
use crate::phase2::puzzle::{OrbitDef, slice_puzzle::exact_hasher_orbit_bytes};
use std::num::NonZeroU8;

#[derive(Clone, PartialEq, Debug, Hash)]
pub struct SliceOrbitPuzzle(Box<[u8]>);

impl OrbitPuzzleState for SliceOrbitPuzzle {
    type MultiBv = Box<[u8]>;

    // TODO: make everything here DRY
    fn replace_compose(&mut self, a: &Self, b: &Self, orbit_def: OrbitDef) {
        let piece_count = orbit_def.piece_count.get() as usize;
        let orientation_count = orbit_def.orientation_count;
        // SAFETY: Permutation vectors and orientation vectors are shuffled
        // around, based on code from twsearch [1]. Testing has shown this is
        // sound.
        // [1] https://github.com/cubing/twsearch
        if orientation_count == 1.try_into().unwrap() {
            for i in 0..piece_count {
                unsafe {
                    let pos = *a.0.get_unchecked(*b.0.get_unchecked(i) as usize);
                    *self.0.get_unchecked_mut(i) = pos;
                    *self.0.get_unchecked_mut(i + piece_count) = 0;
                }
            }
        } else {
            for i in 0..piece_count {
                unsafe {
                    let pos = a.0.get_unchecked(*b.0.get_unchecked(i) as usize);
                    let a_ori =
                        a.0.get_unchecked(*b.0.get_unchecked(i) as usize + piece_count);
                    let b_ori = b.0.get_unchecked(i + piece_count);
                    *self.0.get_unchecked_mut(i) = *pos;
                    *self.0.get_unchecked_mut(i + piece_count) =
                        (*a_ori + *b_ori) % orientation_count;
                }
            }
        }
    }

    fn induces_sorted_orbit_cycle_type(
        &self,
        sorted_orbit_cycle_type: &[(NonZeroU8, bool)],
        orbit_def: OrbitDef,
        multi_bv: &mut [u8],
    ) -> bool {
        multi_bv.fill(0);
        let mut covered_cycles_count = 0;
        let piece_count = orbit_def.piece_count.get() as usize;
        let orientation_count = orbit_def.orientation_count;
        for i in 0..piece_count {
            let (div, rem) = (i / 4, i % 4);
            // SAFETY: default_multi_bv_slice ensures that (i / 4) always fits
            // in multi_bv
            if unsafe { *multi_bv.get_unchecked(div) } & (1 << rem) != 0 {
                continue;
            }
            // SAFETY: see above
            unsafe {
                *multi_bv.get_unchecked_mut(div) |= 1 << rem;
            }
            let mut actual_cycle_length = 1;
            // SAFETY: sorted_orbit_defs guarantees that base (the orbit state
            // base pointer) + i (a permutation vector element) is valid
            let mut piece = unsafe { *self.0.get_unchecked(i) } as usize;
            // SAFETY: sorted_orbit_defs guarantees that base (the orbit state
            // base pointer) + i + piece (an orientation vector element) is valid
            let mut orientation_sum = unsafe { *self.0.get_unchecked(piece + piece_count) };

            while piece != i {
                actual_cycle_length += 1;
                let (div, rem) = (piece / 4, piece % 4);
                // SAFETY: default_multi_bv_slice ensures that (piece / 4)
                // always in fits in multi_bv
                unsafe {
                    *multi_bv.get_unchecked_mut(div) |= 1 << rem;
                }
                // SAFETY: sorted_orbit_defs guarantees that base (the orbit
                // state base pointer) + piece (a permutation vector element) is
                // valid
                unsafe {
                    piece = *self.0.get_unchecked(piece) as usize;
                }
                // SAFETY: sorted_orbit_defs guarantees that base (the orbit
                // state base pointer) + piece + piece_count (an orientation
                // vector element) is valid
                unsafe {
                    orientation_sum += *self.0.get_unchecked(piece + piece_count);
                }
            }

            let actual_orients = orientation_sum % orientation_count != 0;
            if actual_cycle_length == 1 && !actual_orients {
                continue;
            }
            let Some(valid_cycle_index) = sorted_orbit_cycle_type.iter().enumerate().position(
                |(j, &(expected_cycle_length, expected_orients))| {
                    let (div, rem) = (j / 4, j % 4);
                    expected_cycle_length.get() == actual_cycle_length
                        && expected_orients == actual_orients
                        // SAFETY: default_multi_bv_slice ensures that (j / 4)
                        // always fits in multi_bv
                        && unsafe { *multi_bv.get_unchecked(div) } & (1 << (rem + 4)) == 0
                },
            ) else {
                return false;
            };
            let (div, rem) = (valid_cycle_index / 4, valid_cycle_index % 4);
            // SAFETY: default_multi_bv_slice ensures that
            // (valid_cycle_index / 4) always fits in multi_bv
            unsafe {
                *multi_bv.get_unchecked_mut(div) |= 1 << (rem + 4);
            }
            covered_cycles_count += 1;
            // cannot possibly return true if this runs
            if covered_cycles_count > sorted_orbit_cycle_type.len() {
                return false;
            }
        }
        covered_cycles_count == sorted_orbit_cycle_type.len()
    }

    #[allow(refining_impl_trait)]
    fn approximate_hash(&self) -> &Self {
        self
    }

    fn exact_hasher(&self, orbit_def: OrbitDef) -> u64 {
        let (perm, ori) = self.0.split_at(orbit_def.piece_count.get() as usize);
        exact_hasher_orbit_bytes(perm, ori, orbit_def)
    }
}

impl OrbitPuzzleConstructors for SliceOrbitPuzzle {
    type MultiBv = Box<[u8]>;

    fn new_multi_bv(orbit_def: OrbitDef) -> Box<[u8]> {
        vec![0; orbit_def.piece_count.get().div_ceil(4) as usize].into_boxed_slice()
    }

    fn from_orbit_transformation_unchecked<B: AsRef<[u8]>>(
        perm: B,
        ori: B,
        orbit_def: OrbitDef,
    ) -> Self {
        let mut orbit_states = vec![0_u8; orbit_def.piece_count.get() as usize * 2];
        let piece_count = orbit_def.piece_count.get();
        for i in 0..piece_count {
            orbit_states[(piece_count + i) as usize] = ori.as_ref()[i as usize];
            orbit_states[i as usize] = perm.as_ref()[i as usize];
        }
        SliceOrbitPuzzle(orbit_states.into_boxed_slice())
    }
}
