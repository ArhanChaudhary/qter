//! The default, generic implementation for representing puzzle states.

use super::{
    BrandedOrbitDef, OrbitIdentifier, PuzzleState, SliceViewMut, SortedOrbitDefsRef,
    TransformationsMeta, TransformationsMetaError,
};
use crate::{
    FACT_UNTIL_19, SliceView,
    orbit_puzzle::{
        OrbitPuzzleStateImplementor,
        slice_orbit_puzzle::{
            SliceOrbitPuzzle, induces_sorted_cycle_type_slice_orbit, replace_compose_slice_orbit,
        },
    },
    puzzle::{AuxMem, AuxMemRefMut, OrbitDef, SortedCycleType, SortedCycleTypeRef},
};
use generativity::Id;
use itertools::Itertools;
use std::{hint::assert_unchecked, slice};

#[derive(Clone, PartialEq, Debug)]
pub struct StackPuzzle<'id, const N: usize>([u8; N], Id<'id>);

#[derive(Clone, PartialEq, Debug)]
pub struct HeapPuzzle<'id>(Box<[u8]>, Id<'id>);

/// A newtyped index into the start of an orbit in a `StackPuzzle` or
/// `HeapPuzzle`.
#[derive(Clone, Copy, Debug)]
pub struct SliceOrbitIdentifier<'id> {
    base_index: usize,
    pub(crate) branded_orbit_def: BrandedOrbitDef<'id>,
}

impl<'id> OrbitIdentifier<'id> for SliceOrbitIdentifier<'id> {
    fn first_orbit_identifier(branded_orbit_def: BrandedOrbitDef<'id>) -> Self {
        SliceOrbitIdentifier {
            base_index: 0,
            branded_orbit_def,
        }
    }

    fn next_orbit_identifier(
        self,
        branded_orbit_def: BrandedOrbitDef<'id>,
    ) -> SliceOrbitIdentifier<'id> {
        // TODO: panic if out of bounds
        SliceOrbitIdentifier {
            base_index: self.base_index
                + self.branded_orbit_def.inner.piece_count.get() as usize * 2,
            branded_orbit_def,
        }
    }

    fn orbit_def(&self) -> OrbitDef {
        self.branded_orbit_def.inner
    }
}

// TODO: should this be unsafe
impl SliceOrbitIdentifier<'_> {
    #[must_use]
    pub fn base_index(self) -> usize {
        self.base_index
    }

    #[must_use]
    pub fn perm_slice(self, slice_orbit_states: &[u8]) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                slice_orbit_states.as_ptr().add(self.base_index),
                self.branded_orbit_def.inner.piece_count.get() as usize,
            )
        }
    }

    #[must_use]
    pub fn ori_slice(self, slice_orbit_states: &[u8]) -> &[u8] {
        let start = self.base_index + self.branded_orbit_def.inner.piece_count.get() as usize;
        unsafe {
            slice::from_raw_parts(
                slice_orbit_states.as_ptr().add(start),
                self.branded_orbit_def.inner.piece_count.get() as usize,
            )
        }
    }

    #[must_use]
    pub fn orbit_slice(self, slice_orbit_states: &[u8]) -> &[u8] {
        let start = self.base_index;
        let end = self
            .next_orbit_identifier(self.branded_orbit_def)
            .base_index();
        unsafe { slice::from_raw_parts(slice_orbit_states.as_ptr().add(start), end - start) }
    }
}

#[must_use]
pub fn slice_orbit_size(orbit_def: OrbitDef) -> usize {
    orbit_def.piece_count.get() as usize * 2
}

impl<'id, const N: usize> PuzzleState<'id> for StackPuzzle<'id, N> {
    type OrbitBytesBuf<'a>
        = &'a [u8]
    where
        Self: 'a;
    type OrbitIdentifier = SliceOrbitIdentifier<'id>;

    fn new_aux_mem(sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>) -> AuxMem<'id> {
        new_aux_mem_slice(sorted_orbit_defs)
    }

    fn try_from_transformations_meta(
        transformations_meta: TransformationsMeta<'id, '_>,
        id: Id<'id>,
    ) -> Result<Self, TransformationsMetaError> {
        let mut slice_orbit_states = [0_u8; N];
        ksolve_move_to_slice(&mut slice_orbit_states, transformations_meta)?;
        Ok(StackPuzzle(slice_orbit_states, id))
    }

    fn replace_compose(
        &mut self,
        a: &StackPuzzle<N>,
        b: &StackPuzzle<N>,
        sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
    ) {
        unsafe {
            replace_compose_slice(&mut self.0, &a.0, &b.0, sorted_orbit_defs);
        }
    }

    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>) {
        unsafe { replace_inverse_slice(&mut self.0, &a.0, sorted_orbit_defs) };
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: SortedCycleTypeRef<'id, '_>,
        sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
        aux_mem: AuxMemRefMut<'id, '_>,
    ) -> bool {
        unsafe {
            induces_sorted_cycle_type_slice(
                &self.0,
                sorted_cycle_type,
                sorted_orbit_defs,
                aux_mem.inner.unwrap_unchecked(),
            )
        }
    }

    fn orbit_bytes(&self, orbit_identifier: SliceOrbitIdentifier<'id>) -> (&[u8], &[u8]) {
        orbit_bytes_slice(&self.0, orbit_identifier)
    }

    fn approximate_hash_orbit(&self, orbit_identifier: SliceOrbitIdentifier<'id>) -> &[u8] {
        approximate_hash_orbit_slice(&self.0, orbit_identifier)
    }

    fn exact_hasher_orbit(&self, orbit_identifier: SliceOrbitIdentifier<'id>) -> u64 {
        let (perm, ori) = self.orbit_bytes(orbit_identifier);
        unsafe { exact_hasher_orbit_bytes(perm, ori, orbit_identifier.orbit_def()) }
    }

    fn pick_orbit_puzzle(orbit_identifier: Self::OrbitIdentifier) -> OrbitPuzzleStateImplementor {
        pick_orbit_puzzle_slice(orbit_identifier)
    }
}

impl<'id> PuzzleState<'id> for HeapPuzzle<'id> {
    type OrbitBytesBuf<'a>
        = &'a [u8]
    where
        Self: 'a;
    type OrbitIdentifier = SliceOrbitIdentifier<'id>;

    fn new_aux_mem(sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>) -> AuxMem<'id> {
        new_aux_mem_slice(sorted_orbit_defs)
    }

    fn try_from_transformations_meta(
        transformations_meta: TransformationsMeta<'id, '_>,
        id: Id<'id>,
    ) -> Result<Self, TransformationsMetaError> {
        let sorted_orbit_defs = transformations_meta.sorted_orbit_defs();
        let mut slice_orbit_states = vec![
            0_u8;
            sorted_orbit_defs
                .branded_copied_iter()
                .map(|branded_orbit_def| slice_orbit_size(branded_orbit_def.inner))
                .sum::<usize>()
        ]
        .into_boxed_slice();
        // No validation needed. from_sorted_transformations_unchecked creates
        // an orbit states buffer that is guaranteed to be the right size, and
        // there is no restriction on the expected orbit defs
        ksolve_move_to_slice(&mut slice_orbit_states, transformations_meta).unwrap();
        Ok(HeapPuzzle(slice_orbit_states, id))
    }

    fn replace_compose(
        &mut self,
        a: &HeapPuzzle,
        b: &HeapPuzzle,
        sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
    ) {
        // SAFETY: the caller guarantees that all arguments correspond to the
        // same orbit defs
        unsafe {
            replace_compose_slice(&mut self.0, &a.0, &b.0, sorted_orbit_defs);
        }
    }

    fn replace_inverse(&mut self, a: &Self, sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>) {
        unsafe { replace_inverse_slice(&mut self.0, &a.0, sorted_orbit_defs) };
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: SortedCycleTypeRef<'id, '_>,
        sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
        aux_mem: AuxMemRefMut<'id, '_>,
    ) -> bool {
        unsafe {
            induces_sorted_cycle_type_slice(
                &self.0,
                sorted_cycle_type,
                sorted_orbit_defs,
                aux_mem.inner.unwrap_unchecked(),
            )
        }
    }

    fn orbit_bytes(&self, orbit_identifier: SliceOrbitIdentifier<'id>) -> (&[u8], &[u8]) {
        orbit_bytes_slice(&self.0, orbit_identifier)
    }

    fn approximate_hash_orbit(&self, orbit_identifier: SliceOrbitIdentifier<'id>) -> &[u8] {
        approximate_hash_orbit_slice(&self.0, orbit_identifier)
    }

    fn exact_hasher_orbit(&self, orbit_identifier: SliceOrbitIdentifier<'id>) -> u64 {
        let (perm, ori) = self.orbit_bytes(orbit_identifier);
        unsafe { exact_hasher_orbit_bytes(perm, ori, orbit_identifier.orbit_def()) }
    }

    fn pick_orbit_puzzle(orbit_identifier: Self::OrbitIdentifier) -> OrbitPuzzleStateImplementor {
        pick_orbit_puzzle_slice(orbit_identifier)
    }
}

/// Create a new multi-bit vector for slice puzzles in
/// `induces_sorted_cycle_type`.
fn new_aux_mem_slice<'id>(sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>) -> AuxMem<'id> {
    AuxMem {
        inner: Some(
            vec![
                0;
                sorted_orbit_defs
                    .inner
                    .last()
                    .unwrap()
                    .piece_count
                    .get()
                    .div_ceil(4) as usize
            ]
            .into_boxed_slice(),
        ),
        id: sorted_orbit_defs.id,
    }
}

impl<'id> From<BrandedOrbitDef<'id>> for AuxMem<'id> {
    fn from(branded_orbit_def: BrandedOrbitDef<'id>) -> Self {
        AuxMem {
            inner: Some(
                vec![0; branded_orbit_def.inner.piece_count.get().div_ceil(4) as usize]
                    .into_boxed_slice(),
            ),
            id: branded_orbit_def.id(),
        }
    }
}

/// Populate `slice_orbit_states` with `transformation_metas`.
fn ksolve_move_to_slice(
    slice_orbit_states: &mut [u8],
    transformations_meta: TransformationsMeta,
) -> Result<(), TransformationsMetaError> {
    let sorted_orbit_defs = transformations_meta.sorted_orbit_defs();

    if slice_orbit_states.len()
        < sorted_orbit_defs
            .branded_copied_iter()
            .map(|branded_orbit_def| slice_orbit_size(branded_orbit_def.inner))
            .sum()
    {
        return Err(TransformationsMetaError::NotEnoughBufferSpace);
    }
    let sorted_transformations = transformations_meta.sorted_transformations();
    let mut base = 0;
    for (transformation, branded_orbit_def) in sorted_transformations
        .iter()
        .zip(sorted_orbit_defs.branded_copied_iter())
    {
        let piece_count = branded_orbit_def.inner.piece_count.get();
        // TODO: make this more efficient:
        // - zero orientation mod optimization (change next_orbit_identifier_slice too)
        // - avoid the transformation for identities entirely
        for i in 0..piece_count {
            let (perm, orientation_delta) = transformation[i as usize];
            slice_orbit_states[base + (i + piece_count) as usize] = orientation_delta;
            slice_orbit_states[base + i as usize] = perm;
        }
        base += slice_orbit_size(branded_orbit_def.inner);
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
    sorted_orbit_defs: SortedOrbitDefsRef,
) {
    debug_assert_eq!(
        sorted_orbit_defs
            .branded_copied_iter()
            .map(|branded_orbit_def| slice_orbit_size(branded_orbit_def.inner))
            .sum::<usize>(),
        slice_orbit_states_mut.len()
    );
    debug_assert_eq!(slice_orbit_states_mut.len(), a.len());
    debug_assert_eq!(a.len(), b.len());

    let mut base = 0;
    for branded_orbit_def in sorted_orbit_defs.branded_copied_iter() {
        unsafe {
            replace_compose_slice_orbit(
                slice_orbit_states_mut,
                base,
                a,
                b,
                branded_orbit_def.inner,
            );
        }
        base += slice_orbit_size(branded_orbit_def.inner);
    }
}

unsafe fn replace_inverse_slice(
    slice_orbit_states_mut: &mut [u8],
    a: &[u8],
    sorted_orbit_defs: SortedOrbitDefsRef,
) {
    debug_assert_eq!(
        sorted_orbit_defs
            .branded_copied_iter()
            .map(|branded_orbit_def| slice_orbit_size(branded_orbit_def.inner))
            .sum::<usize>(),
        slice_orbit_states_mut.len()
    );
    debug_assert_eq!(slice_orbit_states_mut.len(), a.len());

    let mut base = 0;
    for branded_orbit_def in sorted_orbit_defs.branded_copied_iter() {
        let piece_count = branded_orbit_def.inner.piece_count.get();
        let orientation_count = branded_orbit_def.inner.orientation_count.get();
        // SAFETY: Permutation vectors and orientation vectors are shuffled
        // around, based on code from twsearch [1]. Testing has shown this is
        // sound.
        //
        // [1] https://github.com/cubing/twsearch
        if orientation_count == 1 {
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
                        (orientation_count - a[base_i + piece_count as usize])
                            .min(a[base_i + piece_count as usize].wrapping_neg());
                }
            }
        }
        base += slice_orbit_size(branded_orbit_def.inner);
    }
}

#[allow(clippy::needless_pass_by_value)]
#[inline]
unsafe fn induces_sorted_cycle_type_slice<'id>(
    slice_orbit_states: &[u8],
    sorted_cycle_type: SortedCycleTypeRef<'id, '_>,
    sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
    // must be from `new_aux_mem` and inner cannot be None
    aux_mem: &mut [u8],
) -> bool {
    unsafe {
        assert_unchecked(sorted_cycle_type.inner.len() == sorted_cycle_type.inner.len());
    }
    let mut base = 0;
    for (branded_orbit_def, sorted_cycle_type_orbit) in sorted_orbit_defs
        .branded_copied_iter()
        .zip(sorted_cycle_type.inner.iter())
    {
        unsafe {
            if !induces_sorted_cycle_type_slice_orbit(
                slice_orbit_states,
                base,
                sorted_cycle_type_orbit,
                branded_orbit_def.inner,
                aux_mem,
            ) {
                return false;
            }
        };
        base += slice_orbit_size(branded_orbit_def.inner);
    }
    true
}

fn orbit_bytes_slice<'a>(
    slice_orbit_states: &'a [u8],
    slice_orbit_identifier: SliceOrbitIdentifier,
) -> (&'a [u8], &'a [u8]) {
    (
        slice_orbit_identifier.perm_slice(slice_orbit_states),
        slice_orbit_identifier.ori_slice(slice_orbit_states),
    )
}

fn approximate_hash_orbit_slice<'a>(
    slice_orbit_states: &'a [u8],
    slice_orbit_identifier: SliceOrbitIdentifier,
) -> &'a [u8] {
    slice_orbit_identifier.orbit_slice(slice_orbit_states)
}

// TODO: https://stackoverflow.com/a/24689277 https://freedium.cfd/https://medium.com/@benjamin.botto/sequentially-indexing-permutations-a-linear-algorithm-for-computing-lexicographic-rank-a22220ffd6e3 https://stackoverflow.com/questions/1506078/fast-permutation-number-permutation-mapping-algorithms/1506337#1506337
pub(crate) unsafe fn exact_hasher_orbit_bytes(perm: &[u8], ori: &[u8], orbit_def: OrbitDef) -> u64 {
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

fn pick_orbit_puzzle_slice(orbit_identifier: SliceOrbitIdentifier) -> OrbitPuzzleStateImplementor {
    let orbit_def = orbit_identifier.orbit_def();
    let perm = (0..orbit_def.piece_count.get()).collect_vec();
    let ori = vec![0; orbit_def.piece_count.get() as usize];
    unsafe { SliceOrbitPuzzle::from_orbit_transformation_unchecked(perm, ori, orbit_def).into() }
}

impl<'id> HeapPuzzle<'id> {
    /// Utility function for testing. Not optimized.
    ///
    /// # Panics
    ///
    /// Panics if the generated cycle type is deemed to be invalid because of
    /// bad implementation of the function.
    #[must_use]
    pub fn sorted_cycle_type(
        &self,
        sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
        aux_mem: &mut AuxMem<'id>,
    ) -> SortedCycleType<'id> {
        let aux_mem_inner = aux_mem.inner.as_mut().unwrap().as_mut();
        let mut cycle_type = vec![];
        let mut base = 0;
        for branded_orbit_def in sorted_orbit_defs.branded_copied_iter() {
            let mut cycle_type_piece = vec![];
            aux_mem_inner.fill(0);
            let piece_count = branded_orbit_def.inner.piece_count.get() as usize;
            for i in 0..piece_count {
                let (div, rem) = (i / 4, i % 4);
                if aux_mem_inner[div] & (1 << rem) != 0 {
                    continue;
                }

                aux_mem_inner[div] |= 1 << rem;
                let mut actual_cycle_length = 1;
                let mut piece = self.0[base + i] as usize;
                let mut orientation_sum = self.0[base + piece + piece_count];

                while piece != i {
                    actual_cycle_length += 1;
                    let (div, rem) = (piece / 4, piece % 4);
                    aux_mem_inner[div] |= 1 << rem;
                    piece = self.0[base + piece] as usize;
                    orientation_sum += self.0[base + piece + piece_count];
                }

                let actual_orients =
                    orientation_sum % branded_orbit_def.inner.orientation_count != 0;
                if actual_cycle_length != 1 || actual_orients {
                    cycle_type_piece.push((actual_cycle_length, actual_orients));
                }
            }
            base += slice_orbit_size(branded_orbit_def.inner);
            cycle_type_piece.sort_unstable();
            cycle_type.push(cycle_type_piece);
        }
        let sorted_cycle_type = SortedCycleType::new(&cycle_type, sorted_orbit_defs).unwrap();
        // We don't actually need to test this function because we have this
        assert!(self.induces_sorted_cycle_type(
            sorted_cycle_type.slice_view(),
            sorted_orbit_defs,
            aux_mem.slice_view_mut()
        ));
        sorted_cycle_type
    }
}
