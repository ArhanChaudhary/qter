//! The default, generic implementation for representing puzzle states.

use super::{
    KSolveConversionError, MultiBvInterface, OrbitDef, OrbitIdentifierInterface, OrientedPartition,
    PuzzleState,
};
use crate::phase2::{
    FACT_UNTIL_19,
    orbit_puzzle::slice_orbit_puzzle::{
        induces_sorted_cycle_type_slice_orbit, replace_compose_slice_orbit,
    },
};
use generativity::Id;
use std::num::NonZeroU8;

#[derive(Clone, PartialEq, Debug)]
pub struct StackPuzzle<'id, const N: usize>([u8; N], Id<'id>);

#[derive(Clone, PartialEq, Debug)]
pub struct HeapPuzzle<'id>(Box<[u8]>, Id<'id>);

pub struct SliceMultiBv(Box<[u8]>);
pub struct SliceMultiBvRefMut<'a>(&'a mut [u8]);

impl MultiBvInterface for SliceMultiBv {
    type ReusableRef<'a> = SliceMultiBvRefMut<'a>;

    fn reusable_ref(&mut self) -> Self::ReusableRef<'_> {
        SliceMultiBvRefMut(&mut self.0)
    }
}

pub use private::*;
mod private {
    //! Private module to disallow explicit instantiation of `OrbitBaseSlice`.

    use core::slice;

    use super::{OrbitDef, OrbitIdentifierInterface};

    /// A newtyped index into the start of an orbit in a `StackPuzzle` or
    /// `HeapPuzzle`.
    #[derive(Default, Clone, Copy, Debug)]
    pub struct SliceOrbitBase(usize);

    impl OrbitIdentifierInterface for SliceOrbitBase {
        fn next_orbit_identifier(self, orbit_def: OrbitDef) -> SliceOrbitBase {
            // TODO: panic if out of bounds
            SliceOrbitBase(self.0 + orbit_def.piece_count.get() as usize * 2)
        }
    }

    impl SliceOrbitBase {
        #[must_use]
        pub fn get(self) -> usize {
            self.0
        }

        #[must_use]
        pub fn perm_slice(self, slice_orbit_states: &[u8], orbit_def: OrbitDef) -> &[u8] {
            let start = self.0;
            unsafe {
                slice::from_raw_parts(
                    slice_orbit_states.as_ptr().add(start),
                    orbit_def.piece_count.get() as usize,
                )
            }
        }

        #[must_use]
        pub fn ori_slice(self, slice_orbit_states: &[u8], orbit_def: OrbitDef) -> &[u8] {
            let start = self.0 + orbit_def.piece_count.get() as usize;
            unsafe {
                slice::from_raw_parts(
                    slice_orbit_states.as_ptr().add(start),
                    orbit_def.piece_count.get() as usize,
                )
            }
        }

        #[must_use]
        pub fn orbit_slice(self, slice_orbit_states: &[u8], orbit_def: OrbitDef) -> &[u8] {
            let start = self.0;
            let end = self.next_orbit_identifier(orbit_def).get();
            unsafe { slice::from_raw_parts(slice_orbit_states.as_ptr().add(start), end - start) }
        }
    }
}

#[must_use]
pub fn slice_orbit_size(orbit_def: OrbitDef) -> usize {
    SliceOrbitBase::default()
        .next_orbit_identifier(orbit_def)
        .get()
}

impl<'id, const N: usize> PuzzleState<'id> for StackPuzzle<'id, N> {
    type MultiBv = SliceMultiBv;
    type OrbitBytesBuf<'a>
        = &'a [u8]
    where
        Self: 'a;
    type OrbitIdentifier = SliceOrbitBase;

    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        new_multi_bv_slice(sorted_orbit_defs)
    }

    fn try_from_transformation_meta(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
        id: Id<'id>,
    ) -> Result<Self, KSolveConversionError> {
        let mut slice_orbit_states = [0_u8; N];
        ksolve_move_to_slice(
            &mut slice_orbit_states,
            sorted_transformations,
            sorted_orbit_defs,
        )?;
        Ok(StackPuzzle(slice_orbit_states, id))
    }

    fn replace_compose(
        &mut self,
        a: &StackPuzzle<N>,
        b: &StackPuzzle<N>,
        sorted_orbit_defs: &[OrbitDef],
    ) {
        // SAFETY: the caller guarantees that all arguments correspond to the
        // same orbit defs
        unsafe {
            replace_compose_slice(&mut self.0, &a.0, &b.0, sorted_orbit_defs);
        }
    }

    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: &[OrbitDef]) {
        unsafe { replace_inverse_slice(&mut self.0, &a.0, sorted_orbit_defs) };
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: SliceMultiBvRefMut<'_>,
    ) -> bool {
        induces_sorted_cycle_type_slice(&self.0, sorted_cycle_type, sorted_orbit_defs, multi_bv.0)
    }

    fn orbit_bytes(&self, orbit_identifier: SliceOrbitBase, orbit_def: OrbitDef) -> (&[u8], &[u8]) {
        orbit_bytes_slice(&self.0, orbit_identifier, orbit_def)
    }

    #[allow(refining_impl_trait_reachable)]
    fn approximate_hash_orbit(
        &self,
        orbit_identifier: SliceOrbitBase,
        orbit_def: OrbitDef,
    ) -> &[u8] {
        approximate_hash_orbit_slice(&self.0, orbit_identifier, orbit_def)
    }

    fn exact_hasher_orbit(&self, orbit_identifier: SliceOrbitBase, orbit_def: OrbitDef) -> u64 {
        let (perm, ori) = self.orbit_bytes(orbit_identifier, orbit_def);
        exact_hasher_orbit_bytes(perm, ori, orbit_def)
    }
}

impl<'id> PuzzleState<'id> for HeapPuzzle<'id> {
    type MultiBv = SliceMultiBv;
    type OrbitBytesBuf<'a>
        = &'a [u8]
    where
        Self: 'a;
    type OrbitIdentifier = SliceOrbitBase;

    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        new_multi_bv_slice(sorted_orbit_defs)
    }

    fn try_from_transformation_meta(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
        id: Id<'id>,
    ) -> Result<Self, KSolveConversionError> {
        let mut slice_orbit_states = vec![
            0_u8;
            sorted_orbit_defs
                .iter()
                .copied()
                .map(slice_orbit_size)
                .sum()
        ]
        .into_boxed_slice();
        // No validation needed. from_sorted_transformations_unchecked creates
        // an orbit states buffer that is guaranteed to be the right size, and
        // there is no restriction on the expected orbit defs
        ksolve_move_to_slice(
            &mut slice_orbit_states,
            sorted_transformations,
            sorted_orbit_defs,
        )
        .unwrap();
        Ok(HeapPuzzle(slice_orbit_states, id))
    }

    fn replace_compose(&mut self, a: &HeapPuzzle, b: &HeapPuzzle, sorted_orbit_defs: &[OrbitDef]) {
        // SAFETY: the caller guarantees that all arguments correspond to the
        // same orbit defs
        unsafe {
            replace_compose_slice(&mut self.0, &a.0, &b.0, sorted_orbit_defs);
        }
    }

    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: &[OrbitDef]) {
        unsafe { replace_inverse_slice(&mut self.0, &a.0, sorted_orbit_defs) };
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: SliceMultiBvRefMut<'_>,
    ) -> bool {
        induces_sorted_cycle_type_slice(&self.0, sorted_cycle_type, sorted_orbit_defs, multi_bv.0)
    }

    fn orbit_bytes(&self, orbit_identifier: SliceOrbitBase, orbit_def: OrbitDef) -> (&[u8], &[u8]) {
        orbit_bytes_slice(&self.0, orbit_identifier, orbit_def)
    }

    #[allow(refining_impl_trait_reachable)]
    fn approximate_hash_orbit(
        &self,
        orbit_identifier: SliceOrbitBase,
        orbit_def: OrbitDef,
    ) -> &[u8] {
        approximate_hash_orbit_slice(&self.0, orbit_identifier, orbit_def)
    }

    fn exact_hasher_orbit(&self, orbit_identifier: SliceOrbitBase, orbit_def: OrbitDef) -> u64 {
        let (perm, ori) = self.orbit_bytes(orbit_identifier, orbit_def);
        exact_hasher_orbit_bytes(perm, ori, orbit_def)
    }
}

fn new_multi_bv_slice(sorted_orbit_defs: &[OrbitDef]) -> SliceMultiBv {
    SliceMultiBv(
        vec![
            0;
            sorted_orbit_defs
                .last()
                .unwrap()
                .piece_count
                .get()
                .div_ceil(4) as usize
        ]
        .into_boxed_slice(),
    )
}

fn ksolve_move_to_slice(
    slice_orbit_states: &mut [u8],
    sorted_transformations: &[Vec<(u8, u8)>],
    sorted_orbit_defs: &[OrbitDef],
) -> Result<(), KSolveConversionError> {
    if slice_orbit_states.len()
        < sorted_orbit_defs
            .iter()
            .copied()
            .map(slice_orbit_size)
            .sum()
    {
        return Err(KSolveConversionError::NotEnoughBufferSpace);
    }
    let mut base = 0;
    for (transformation, &orbit_def) in sorted_transformations.iter().zip(sorted_orbit_defs.iter())
    {
        let piece_count = orbit_def.piece_count.get();
        // TODO: make this more efficient:
        // - zero orientation mod optimization (change next_orbit_identifier_slice too)
        // - avoid the transformation for identities entirely
        if transformation.is_empty() {
            for i in 0..piece_count {
                slice_orbit_states[base + (i + piece_count) as usize] = 0;
                slice_orbit_states[base + i as usize] = i;
            }
        } else {
            for i in 0..piece_count {
                let (perm, orientation_delta) = transformation[i as usize];
                slice_orbit_states[base + (i + piece_count) as usize] = orientation_delta;
                slice_orbit_states[base + i as usize] = perm;
            }
        }
        base += slice_orbit_size(orbit_def);
    }
    Ok(())
}

/// # SAFETY
///
/// `slice_orbit_states_mut`, `a`, and `b` must all correspond to `sorted_orbit_defs`.
unsafe fn replace_compose_slice(
    slice_orbit_states_mut: &mut [u8],
    a: &[u8],
    b: &[u8],
    sorted_orbit_defs: &[OrbitDef],
) {
    debug_assert_eq!(
        sorted_orbit_defs
            .iter()
            .copied()
            .map(slice_orbit_size)
            .sum::<usize>(),
        slice_orbit_states_mut.len()
    );
    debug_assert_eq!(slice_orbit_states_mut.len(), a.len());
    debug_assert_eq!(a.len(), b.len());

    let mut base = 0;
    for &orbit_def in sorted_orbit_defs {
        unsafe {
            replace_compose_slice_orbit(slice_orbit_states_mut, base, a, b, orbit_def);
        }
        base += slice_orbit_size(orbit_def);
    }
}

unsafe fn replace_inverse_slice(
    slice_orbit_states_mut: &mut [u8],
    a: &[u8],
    sorted_orbit_defs: &[OrbitDef],
) {
    debug_assert_eq!(
        sorted_orbit_defs
            .iter()
            .copied()
            .map(slice_orbit_size)
            .sum::<usize>(),
        slice_orbit_states_mut.len()
    );
    debug_assert_eq!(slice_orbit_states_mut.len(), a.len());

    let mut base = 0;
    for &orbit_def in sorted_orbit_defs {
        let piece_count = orbit_def.piece_count.get();
        // SAFETY: Permutation vectors and orientation vectors are shuffled
        // around, based on code from twsearch [1]. Testing has shown this is
        // sound.
        //
        // [1] https://github.com/cubing/twsearch
        if orbit_def.orientation_count == 1.try_into().unwrap() {
            for i in 0..piece_count {
                let base_i = base + i as usize;
                unsafe {
                    *slice_orbit_states_mut.get_unchecked_mut(base + a[base_i] as usize) = i;
                    *slice_orbit_states_mut
                        .get_unchecked_mut(base + (a[base_i] + piece_count) as usize) = 0;
                }
            }
        } else {
            for i in 0..piece_count {
                let base_i = base + i as usize;
                unsafe {
                    *slice_orbit_states_mut.get_unchecked_mut(base + (a[base_i]) as usize) = i;
                    *slice_orbit_states_mut
                        .get_unchecked_mut(base + (a[base_i] + piece_count) as usize) =
                        (orbit_def.orientation_count.get() - a[base_i + piece_count as usize])
                            .min(a[base_i + piece_count as usize].wrapping_neg());
                }
            }
        }
        base += slice_orbit_size(orbit_def);
    }
}

#[inline]
fn induces_sorted_cycle_type_slice(
    slice_orbit_states: &[u8],
    sorted_cycle_type: &[OrientedPartition],
    sorted_orbit_defs: &[OrbitDef],
    multi_bv: &mut [u8],
) -> bool {
    let mut base = 0;
    for (&orbit_def, sorted_cycle_type_orbit) in
        sorted_orbit_defs.iter().zip(sorted_cycle_type.iter())
    {
        unsafe {
            if !induces_sorted_cycle_type_slice_orbit(
                slice_orbit_states,
                base,
                sorted_cycle_type_orbit,
                orbit_def,
                multi_bv,
            ) {
                return false;
            }
        };
        base += slice_orbit_size(orbit_def);
    }
    true
}

fn orbit_bytes_slice(
    slice_orbit_states: &[u8],
    slice_orbit_base: SliceOrbitBase,
    orbit_def: OrbitDef,
) -> (&[u8], &[u8]) {
    (
        slice_orbit_base.perm_slice(slice_orbit_states, orbit_def),
        slice_orbit_base.ori_slice(slice_orbit_states, orbit_def),
    )
}

fn approximate_hash_orbit_slice(
    slice_orbit_states: &[u8],
    slice_orbit_base: SliceOrbitBase,
    orbit_def: OrbitDef,
) -> &[u8] {
    slice_orbit_base.orbit_slice(slice_orbit_states, orbit_def)
}

// TODO: https://stackoverflow.com/a/24689277 https://freedium.cfd/https://medium.com/@benjamin.botto/sequentially-indexing-permutations-a-linear-algorithm-for-computing-lexicographic-rank-a22220ffd6e3 https://stackoverflow.com/questions/1506078/fast-permutation-number-permutation-mapping-algorithms/1506337#1506337
pub(crate) fn exact_hasher_orbit_bytes(perm: &[u8], ori: &[u8], orbit_def: OrbitDef) -> u64 {
    let piece_count = orbit_def.piece_count.get();
    assert!(piece_count as usize <= FACT_UNTIL_19.len());

    let mut exact_perm_hash = u64::from(perm[0]) * FACT_UNTIL_19[(piece_count - 1) as usize];
    for i in 1..piece_count - 1 {
        let mut res = 0;
        for j in (i + 1)..piece_count {
            if perm[j as usize] < perm[i as usize] {
                res += 1;
            }
        }
        exact_perm_hash += res * FACT_UNTIL_19[(piece_count - i - 1) as usize];
    }

    // TODO: IMPORTANT: we need parity as a const generic maybe? or an argument
    // see the screenshot

    let mut exact_ori_hash = 0;
    for i in 0..piece_count - 1 {
        exact_ori_hash *= u64::from(orbit_def.orientation_count.get());
        exact_ori_hash += u64::from(ori[i as usize]);
    }

    exact_perm_hash
        * u64::pow(
            u64::from(orbit_def.orientation_count.get()),
            u32::from(piece_count) - 1,
        )
        + exact_ori_hash
}

impl HeapPuzzle<'_> {
    /// Utility function for testing. Not optimized.
    ///
    /// # Panics
    ///
    /// Panics if the generated cycle type is deemed to be invalid because of
    /// bad implementation of the function.
    #[must_use]
    pub fn cycle_type(
        &self,
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: SliceMultiBvRefMut<'_>,
    ) -> Vec<OrientedPartition> {
        let mut cycle_type = vec![];
        let mut base = 0;
        for &orbit_def in sorted_orbit_defs {
            let mut cycle_type_piece = vec![];
            multi_bv.0.fill(0);
            let piece_count = orbit_def.piece_count.get() as usize;
            for i in 0..piece_count {
                let (div, rem) = (i / 4, i % 4);
                if multi_bv.0[div] & (1 << rem) != 0 {
                    continue;
                }

                multi_bv.0[div] |= 1 << rem;
                let mut actual_cycle_length = 1;
                let mut piece = self.0[base + i] as usize;
                let mut orientation_sum = self.0[base + piece + piece_count];

                while piece != i {
                    actual_cycle_length += 1;
                    let (div, rem) = (piece / 4, piece % 4);
                    multi_bv.0[div] |= 1 << rem;
                    piece = self.0[base + piece] as usize;
                    orientation_sum += self.0[base + piece + piece_count];
                }

                let actual_orients = orientation_sum % orbit_def.orientation_count != 0;
                if actual_cycle_length != 1 || actual_orients {
                    cycle_type_piece
                        .push((NonZeroU8::new(actual_cycle_length).unwrap(), actual_orients));
                }
            }
            base += slice_orbit_size(orbit_def);
            cycle_type_piece.sort();
            cycle_type.push(cycle_type_piece);
        }
        // We don't actually need to test this function because we have this
        assert!(self.induces_sorted_cycle_type(&cycle_type, sorted_orbit_defs, multi_bv));
        cycle_type
    }
}
