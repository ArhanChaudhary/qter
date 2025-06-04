//! The default, generic implementation for representing puzzle states.

use super::{KSolveConversionError, OrbitDef, OrientedPartition, PuzzleState};
use crate::phase2::{
    FACT_UNTIL_19,
    orbit_puzzle::slice_orbit_puzzle::{
        induces_sorted_cycle_type_slice_orbit, replace_compose_slice_orbit,
    },
};
use std::num::NonZeroU8;

#[derive(Clone, PartialEq, Debug)]
pub struct StackPuzzle<const N: usize>([u8; N]);

#[derive(Clone, PartialEq, Debug)]
pub struct HeapPuzzle(Box<[u8]>);

impl<const N: usize> PuzzleState for StackPuzzle<N> {
    // TODO: make this newtype to ensure induces sorted cycle type is always
    // sound
    type MultiBv = Box<[u8]>;
    type OrbitBytesBuf<'a> = &'a [u8];

    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        new_multi_bv_slice(sorted_orbit_defs)
    }

    fn try_from_transformation_meta(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError> {
        let mut orbit_states = [0_u8; N];
        ksolve_move_to_slice(&mut orbit_states, sorted_transformations, sorted_orbit_defs)?;
        Ok(StackPuzzle(orbit_states))
    }

    unsafe fn replace_compose(
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
        replace_inverse_slice(&mut self.0, &a.0, sorted_orbit_defs);
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: &mut [u8],
    ) -> bool {
        induces_sorted_cycle_type_slice(&self.0, sorted_cycle_type, sorted_orbit_defs, multi_bv)
    }

    fn next_orbit_identifer(orbit_identifier: usize, orbit_def: OrbitDef) -> usize {
        next_orbit_identifier_slice(orbit_identifier, orbit_def)
    }

    fn orbit_bytes(&self, orbit_identifier: usize, orbit_def: OrbitDef) -> (&[u8], &[u8]) {
        orbit_bytes_slice(&self.0, orbit_identifier, orbit_def)
    }

    #[allow(refining_impl_trait_reachable)]
    fn approximate_hash_orbit(&self, orbit_identifier: usize, orbit_def: OrbitDef) -> &[u8] {
        approximate_hash_orbit_slice(&self.0, orbit_identifier, orbit_def)
    }

    fn exact_hasher_orbit(&self, orbit_identifier: usize, orbit_def: OrbitDef) -> u64 {
        let (perm, ori) = self.orbit_bytes(orbit_identifier, orbit_def);
        exact_hasher_orbit_bytes(perm, ori, orbit_def)
    }
}

impl PuzzleState for HeapPuzzle {
    type MultiBv = Box<[u8]>;
    type OrbitBytesBuf<'a> = &'a [u8];

    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        new_multi_bv_slice(sorted_orbit_defs)
    }

    fn try_from_transformation_meta(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError> {
        let mut orbit_states = vec![
            0_u8;
            sorted_orbit_defs
                .iter()
                .map(|orbit_def| orbit_def.piece_count.get() as usize * 2)
                .sum()
        ]
        .into_boxed_slice();
        // No validation needed. from_sorted_transformations_unchecked creates
        // an orbit states buffer that is guaranteed to be the right size, and
        // there is no restriction on the expected orbit defs
        ksolve_move_to_slice(&mut orbit_states, sorted_transformations, sorted_orbit_defs).unwrap();
        Ok(HeapPuzzle(orbit_states))
    }

    unsafe fn replace_compose(
        &mut self,
        a: &HeapPuzzle,
        b: &HeapPuzzle,
        sorted_orbit_defs: &[OrbitDef],
    ) {
        // SAFETY: the caller guarantees that all arguments correspond to the
        // same orbit defs
        unsafe {
            replace_compose_slice(&mut self.0, &a.0, &b.0, sorted_orbit_defs);
        }
    }

    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: &[OrbitDef]) {
        replace_inverse_slice(&mut self.0, &a.0, sorted_orbit_defs);
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: &mut [u8],
    ) -> bool {
        induces_sorted_cycle_type_slice(&self.0, sorted_cycle_type, sorted_orbit_defs, multi_bv)
    }

    fn next_orbit_identifer(orbit_identifier: usize, orbit_def: OrbitDef) -> usize {
        next_orbit_identifier_slice(orbit_identifier, orbit_def)
    }

    fn orbit_bytes(&self, orbit_identifier: usize, orbit_def: OrbitDef) -> (&[u8], &[u8]) {
        orbit_bytes_slice(&self.0, orbit_identifier, orbit_def)
    }

    #[allow(refining_impl_trait_reachable)]
    fn approximate_hash_orbit(&self, orbit_identifier: usize, orbit_def: OrbitDef) -> &[u8] {
        approximate_hash_orbit_slice(&self.0, orbit_identifier, orbit_def)
    }

    fn exact_hasher_orbit(&self, orbit_identifier: usize, orbit_def: OrbitDef) -> u64 {
        let (perm, ori) = self.orbit_bytes(orbit_identifier, orbit_def);
        exact_hasher_orbit_bytes(perm, ori, orbit_def)
    }
}

fn new_multi_bv_slice(sorted_orbit_defs: &[OrbitDef]) -> Box<[u8]> {
    vec![
        0;
        sorted_orbit_defs
            .last()
            .unwrap()
            .piece_count
            .get()
            .div_ceil(4) as usize
    ]
    .into_boxed_slice()
}

fn ksolve_move_to_slice(
    orbit_states: &mut [u8],
    sorted_transformations: &[Vec<(u8, u8)>],
    sorted_orbit_defs: &[OrbitDef],
) -> Result<(), KSolveConversionError> {
    if orbit_states.len()
        < sorted_orbit_defs
            .iter()
            .map(|orbit_def| orbit_def.piece_count.get() as usize * 2)
            .sum()
    {
        return Err(KSolveConversionError::NotEnoughBufferSpace);
    }
    let mut i = 0;
    for (transformation, orbit_def) in sorted_transformations.iter().zip(sorted_orbit_defs.iter()) {
        let piece_count = orbit_def.piece_count.get();
        // TODO: make this more efficient:
        // - zero orientation mod optimization (change next_orbit_identifier_slice too)
        // - avoid the transformation for identities entirely
        if transformation.is_empty() {
            for j in 0..piece_count {
                orbit_states[(i + j + piece_count) as usize] = 0;
                orbit_states[(i + j) as usize] = j;
            }
        } else {
            for j in 0..piece_count {
                let (perm, orientation_delta) = transformation[j as usize];
                orbit_states[(i + j + piece_count) as usize] = orientation_delta;
                orbit_states[(i + j) as usize] = perm;
            }
        }
        i += piece_count * 2;
    }
    Ok(())
}

/// # SAFETY
///
/// `orbit_states_mut`, `a`, and `b` must all correspond to `sorted_orbit_defs`.
unsafe fn replace_compose_slice(
    orbit_states_mut: &mut [u8],
    a: &[u8],
    b: &[u8],
    sorted_orbit_defs: &[OrbitDef],
) {
    debug_assert_eq!(
        sorted_orbit_defs
            .iter()
            .map(|orbit_def| orbit_def.piece_count.get() as usize * 2)
            .sum::<usize>(),
        orbit_states_mut.len()
    );
    debug_assert_eq!(orbit_states_mut.len(), a.len());
    debug_assert_eq!(a.len(), b.len());

    let mut base = 0;
    for &orbit_def in sorted_orbit_defs {
        unsafe {
            replace_compose_slice_orbit(orbit_states_mut, base, a, b, orbit_def);
        }
        base += orbit_def.piece_count.get() as usize * 2;
    }
}

fn replace_inverse_slice(orbit_states_mut: &mut [u8], a: &[u8], sorted_orbit_defs: &[OrbitDef]) {
    debug_assert_eq!(
        sorted_orbit_defs
            .iter()
            .map(|orbit_def| (orbit_def.piece_count.get() as usize) * 2)
            .sum::<usize>(),
        orbit_states_mut.len()
    );
    debug_assert_eq!(orbit_states_mut.len(), a.len());

    let mut base = 0;
    for &OrbitDef {
        piece_count,
        orientation_count,
    } in sorted_orbit_defs
    {
        let piece_count = piece_count.get();
        // SAFETY: Permutation vectors and orientation vectors are shuffled
        // around, based on code from twsearch [1]. Testing has shown this is
        // sound.
        // [1] https://github.com/cubing/twsearch
        if orientation_count == 1.try_into().unwrap() {
            for i in 0..piece_count {
                let base_i = (base + i) as usize;
                unsafe {
                    *orbit_states_mut.get_unchecked_mut((base + a[base_i]) as usize) = i;
                    *orbit_states_mut
                        .get_unchecked_mut((base + a[base_i] + piece_count) as usize) = 0;
                }
            }
        } else {
            for i in 0..piece_count {
                let base_i = (base + i) as usize;
                unsafe {
                    *orbit_states_mut.get_unchecked_mut((base + a[base_i]) as usize) = i;
                    *orbit_states_mut
                        .get_unchecked_mut((base + a[base_i] + piece_count) as usize) =
                        (orientation_count.get() - a[base_i + piece_count as usize])
                            .min(a[base_i + piece_count as usize].wrapping_neg());
                }
            }
        }
        base += piece_count * 2;
    }
}

fn induces_sorted_cycle_type_slice(
    orbit_states: &[u8],
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
                orbit_states,
                base,
                sorted_cycle_type_orbit,
                orbit_def,
                multi_bv,
            ) {
                return false;
            }
        };
        base += orbit_def.piece_count.get() as usize * 2;
    }
    true
}

fn next_orbit_identifier_slice(orbit_base_slice: usize, orbit_def: OrbitDef) -> usize {
    orbit_base_slice + orbit_def.piece_count.get() as usize * 2
}

fn orbit_bytes_slice(
    orbit_states: &[u8],
    orbit_base_slice: usize,
    orbit_def: OrbitDef,
) -> (&[u8], &[u8]) {
    let piece_count = orbit_def.piece_count.get() as usize;
    (
        &orbit_states[orbit_base_slice..orbit_base_slice + piece_count],
        &orbit_states[orbit_base_slice + piece_count
            ..next_orbit_identifier_slice(orbit_base_slice, orbit_def)],
    )
}

fn approximate_hash_orbit_slice(
    orbit_states: &[u8],
    orbit_base_slice: usize,
    orbit_def: OrbitDef,
) -> &[u8] {
    &orbit_states[orbit_base_slice..next_orbit_identifier_slice(orbit_base_slice, orbit_def)]
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

impl HeapPuzzle {
    /// Utility function for testing. Not optimized.
    ///
    /// # Panics
    ///
    /// Panics if the generated cycle type is deemed to be invalid because of
    /// bad implementation of the function.
    pub fn cycle_type(
        &self,
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: &mut [u8],
    ) -> Vec<OrientedPartition> {
        let mut cycle_type = vec![];
        let mut base = 0;
        for &OrbitDef {
            piece_count,
            orientation_count,
        } in sorted_orbit_defs
        {
            let mut cycle_type_piece = vec![];
            multi_bv.fill(0);
            let piece_count = piece_count.get() as usize;
            for i in 0..piece_count {
                let (div, rem) = (i / 4, i % 4);
                if multi_bv[div] & (1 << rem) != 0 {
                    continue;
                }

                multi_bv[div] |= 1 << rem;
                let mut actual_cycle_length = 1;
                let mut piece = self.0[base + i] as usize;
                let mut orientation_sum = self.0[base + piece + piece_count];

                while piece != i {
                    actual_cycle_length += 1;
                    let (div, rem) = (piece / 4, piece % 4);
                    multi_bv[div] |= 1 << rem;
                    piece = self.0[base + piece] as usize;
                    orientation_sum += self.0[base + piece + piece_count];
                }

                let actual_orients = orientation_sum % orientation_count != 0;
                if actual_cycle_length != 1 || actual_orients {
                    cycle_type_piece
                        .push((NonZeroU8::new(actual_cycle_length).unwrap(), actual_orients));
                }
            }
            base += piece_count * 2;
            cycle_type_piece.sort();
            cycle_type.push(cycle_type_piece);
        }
        // We don't actually need to test this function because we have this
        assert!(self.induces_sorted_cycle_type(&cycle_type, sorted_orbit_defs, multi_bv));
        cycle_type
    }
}
