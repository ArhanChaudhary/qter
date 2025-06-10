//! The default and fallback implementation for 3x3 orbits during pruning table
//! generation.

use super::{OrbitPuzzleConstructors, OrbitPuzzleState};
use crate::phase2::puzzle::{MultiBvInterface, OrbitDef, slice_puzzle::exact_hasher_orbit_bytes};
use generativity::Id;
use std::{cmp::Ordering, num::NonZeroU8};

#[derive(Clone, PartialEq, Debug, Hash)]
pub struct SliceOrbitPuzzle<'id>(Box<[u8]>, Id<'id>);

pub struct SliceOrbitMultiBv(Box<[u8]>);
pub struct SliceOrbitMultiBvRefMut<'a>(&'a mut [u8]);

impl MultiBvInterface for SliceOrbitMultiBv {
    type ReusableRef<'a> = SliceOrbitMultiBvRefMut<'a>;

    fn reusable_ref(&mut self) -> Self::ReusableRef<'_> {
        SliceOrbitMultiBvRefMut(&mut self.0)
    }
}

impl OrbitPuzzleState for SliceOrbitPuzzle<'_> {
    type MultiBv = SliceOrbitMultiBv;

    fn replace_compose(&mut self, a: &Self, b: &Self, orbit_def: OrbitDef) {
        unsafe { replace_compose_slice_orbit(&mut self.0, 0, &a.0, &b.0, orbit_def) };
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type_orbit: &[(NonZeroU8, bool)],
        orbit_def: OrbitDef,
        multi_bv: SliceOrbitMultiBvRefMut<'_>,
    ) -> bool {
        // TODO
        unsafe {
            induces_sorted_cycle_type_slice_orbit(
                &self.0,
                0,
                sorted_cycle_type_orbit,
                orbit_def,
                multi_bv.0,
            )
        }
    }

    #[allow(refining_impl_trait)]
    fn approximate_hash(&self) -> &Self {
        self
    }

    fn exact_hasher(&self, orbit_def: OrbitDef) -> u64 {
        // TODO
        let (perm, ori) = unsafe {
            self.0
                .split_at_unchecked(orbit_def.piece_count.get() as usize)
        };
        exact_hasher_orbit_bytes(perm, ori, orbit_def)
    }
}

impl<'id> OrbitPuzzleConstructors<'id> for SliceOrbitPuzzle<'id> {
    type MultiBv = SliceOrbitMultiBv;

    fn new_multi_bv(orbit_def: OrbitDef) -> SliceOrbitMultiBv {
        SliceOrbitMultiBv(
            vec![0; orbit_def.piece_count.get().div_ceil(4) as usize].into_boxed_slice(),
        )
    }

    fn from_orbit_transformation_unchecked<B: AsRef<[u8]>>(
        perm: B,
        ori: B,
        orbit_def: OrbitDef,
        id: Id<'id>,
    ) -> Self {
        let mut slice_orbit_states = vec![0_u8; orbit_def.piece_count.get() as usize * 2];
        let piece_count = orbit_def.piece_count.get();
        for i in 0..piece_count {
            slice_orbit_states[(piece_count + i) as usize] = ori.as_ref()[i as usize];
            slice_orbit_states[i as usize] = perm.as_ref()[i as usize];
        }
        SliceOrbitPuzzle(slice_orbit_states.into_boxed_slice(), id)
    }
}

#[allow(clippy::missing_panics_doc)]
#[inline]
pub unsafe fn replace_compose_slice_orbit(
    slice_orbit_states_mut: &mut [u8],
    base: usize,
    a: &[u8],
    b: &[u8],
    orbit_def: OrbitDef,
) {
    let piece_count = orbit_def.piece_count.get() as usize;
    // SAFETY: Permutation vectors and orientation vectors are shuffled
    // around, based on code from twsearch [1]. Testing has shown this is
    // sound.
    // [1] https://github.com/cubing/twsearch
    if orbit_def.orientation_count == 1.try_into().unwrap() {
        for i in 0..piece_count {
            let base_i = base + i;
            // SAFETY: the caller guarantees that everything is in bounds.
            // Testing has shown this is sound.
            unsafe {
                let pos = *a.get_unchecked(base + *b.get_unchecked(base_i) as usize);
                *slice_orbit_states_mut.get_unchecked_mut(base_i) = pos;
                *slice_orbit_states_mut.get_unchecked_mut(base_i + piece_count) = 0;
            }
        }
    } else {
        for i in 0..piece_count {
            let base_i = base + i;
            // SAFETY: the caller guarantees that everything is in bounds.
            // Testing has shown this is sound.
            unsafe {
                let pos = *a.get_unchecked(base + *b.get_unchecked(base_i) as usize);
                let a_ori =
                    *a.get_unchecked(base + *b.get_unchecked(base_i) as usize + piece_count);
                let b_ori = *b.get_unchecked(base_i + piece_count);
                *slice_orbit_states_mut.get_unchecked_mut(base_i) = pos;
                *slice_orbit_states_mut.get_unchecked_mut(base_i + piece_count) = (a_ori + b_ori)
                    .min((a_ori + b_ori).wrapping_sub(orbit_def.orientation_count.get()));
            }
        }
    }
}

#[inline]
pub unsafe fn induces_sorted_cycle_type_slice_orbit(
    slice_orbit_state: &[u8],
    base: usize,
    sorted_cycle_type_orbit: &[(NonZeroU8, bool)],
    orbit_def: OrbitDef,
    multi_bv: &mut [u8],
) -> bool {
    multi_bv.fill(0);
    let mut covered_cycles_count = 0;
    let piece_count = orbit_def.piece_count.get() as usize;
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
        let mut piece = unsafe { *slice_orbit_state.get_unchecked(base + i) } as usize;
        // SAFETY: sorted_orbit_defs guarantees that base (the orbit state
        // base pointer) + i + piece (an orientation vector element) is valid
        let mut orientation_sum =
            unsafe { *slice_orbit_state.get_unchecked(base + piece + piece_count) };

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
                piece = *slice_orbit_state.get_unchecked(base + piece) as usize;
            }
            // SAFETY: sorted_orbit_defs guarantees that base (the orbit
            // state base pointer) + piece + piece_count (an orientation
            // vector element) is valid
            unsafe {
                orientation_sum += *slice_orbit_state.get_unchecked(base + piece + piece_count);
            }
        }

        let actual_orients = orientation_sum % orbit_def.orientation_count != 0;
        if actual_cycle_length == 1 && !actual_orients {
            continue;
        }
        let mut valid_cycle_index = None;
        for (j, &(expected_cycle_length, expected_orients)) in
            sorted_cycle_type_orbit.iter().enumerate()
        {
            match expected_cycle_length.get().cmp(&actual_cycle_length) {
                Ordering::Less => (),
                Ordering::Equal => {
                    let (div, rem) = (j / 4, j % 4);
                    if expected_orients == actual_orients
                        // SAFETY: default_multi_bv_slice ensures that (j / 4)
                        // always fits in multi_bv
                        && unsafe { *multi_bv.get_unchecked(div) } & (1 << (rem + 4)) == 0
                    {
                        valid_cycle_index = Some(j);
                        break;
                    }
                }
                Ordering::Greater => return false,
            }
        }
        let Some(valid_cycle_index) = valid_cycle_index else {
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
        if covered_cycles_count > sorted_cycle_type_orbit.len() {
            return false;
        }
    }
    covered_cycles_count == sorted_cycle_type_orbit.len()
}
