use std::num::NonZeroU8;

use super::{KSolveConversionError, OrbitDef, OrientedPartition, PuzzleState};

#[derive(Clone, PartialEq, Debug, Hash)]
pub struct StackPuzzle<const N: usize>([u8; N]);

#[derive(Clone, PartialEq, Debug, Hash)]
pub struct HeapPuzzle(Box<[u8]>);

impl<const N: usize> PuzzleState for StackPuzzle<N> {
    type MultiBv = Box<[u8]>;

    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        new_multi_bv_slice(sorted_orbit_defs)
    }

    fn try_from_transformation_meta(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError> {
        if N < sorted_orbit_defs
            .iter()
            .map(|orbit_def| (orbit_def.piece_count.get() as usize) * 2)
            .sum()
        {
            return Err(KSolveConversionError::NotEnoughBufferSpace);
        }

        let mut orbit_states = [0_u8; N];
        ksolve_move_to_slice_unchecked(
            &mut orbit_states,
            sorted_orbit_defs,
            sorted_transformations,
        );
        Ok(StackPuzzle(orbit_states))
    }

    fn replace_compose(
        &mut self,
        a: &StackPuzzle<N>,
        b: &StackPuzzle<N>,
        sorted_orbit_defs: &[OrbitDef],
    ) {
        replace_compose_slice(&mut self.0, &a.0, &b.0, sorted_orbit_defs);
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
}

impl PuzzleState for HeapPuzzle {
    type MultiBv = Box<[u8]>;

    fn new_multi_bv(sorted_orbit_defs: &[OrbitDef]) -> Self::MultiBv {
        new_multi_bv_slice(sorted_orbit_defs)
    }

    fn try_from_transformation_meta(
        sorted_transformations: &[Vec<(u8, u8)>],
        sorted_orbit_defs: &[OrbitDef],
    ) -> Result<Self, KSolveConversionError> {
        // No validation needed. from_sorted_transformations_unchecked creates
        // an orbit states buffer that is guaranteed to be the right size, and
        // there is no restriction on the expected orbit defs
        let mut orbit_states = vec![
            0_u8;
            sorted_orbit_defs
                .iter()
                .map(|orbit_def| orbit_def.piece_count.get() as usize * 2)
                .sum()
        ]
        .into_boxed_slice();
        ksolve_move_to_slice_unchecked(
            &mut orbit_states,
            sorted_orbit_defs,
            sorted_transformations,
        );
        Ok(HeapPuzzle(orbit_states))
    }

    fn replace_compose(&mut self, a: &HeapPuzzle, b: &HeapPuzzle, sorted_orbit_defs: &[OrbitDef]) {
        replace_compose_slice(&mut self.0, &a.0, &b.0, sorted_orbit_defs);
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

fn ksolve_move_to_slice_unchecked(
    orbit_states: &mut [u8],
    sorted_orbit_defs: &[OrbitDef],
    sorted_transformations: &[Vec<(u8, u8)>],
) {
    let mut i = 0;
    for (transformation, orbit_def) in sorted_transformations.iter().zip(sorted_orbit_defs.iter()) {
        let piece_count = orbit_def.piece_count.get() as usize;
        // TODO: make this more efficient:
        // - zero orientation mod optimization (change next_orbit_identifier_slice too)
        // - avoid the transformation for identities entirely
        if transformation.is_empty() {
            for j in 0..piece_count {
                orbit_states[i + j + piece_count] = 0;
                orbit_states[i + j] = j as u8;
            }
        } else {
            for j in 0..piece_count {
                let (perm, orientation_delta) = transformation[j];
                orbit_states[i + j + piece_count] = orientation_delta;
                orbit_states[i + j] = perm;
            }
        }
        i += piece_count * 2;
    }
}

fn replace_compose_slice(
    orbit_states_mut: &mut [u8],
    a: &[u8],
    b: &[u8],
    sorted_orbit_defs: &[OrbitDef],
) {
    debug_assert_eq!(
        sorted_orbit_defs
            .iter()
            .map(|orbit_def| (orbit_def.piece_count.get() as usize) * 2)
            .sum::<usize>(),
        orbit_states_mut.len()
    );
    debug_assert_eq!(orbit_states_mut.len(), a.len());
    debug_assert_eq!(a.len(), b.len());

    let mut base = 0;
    for &OrbitDef {
        piece_count,
        orientation_count,
    } in sorted_orbit_defs
    {
        let piece_count = piece_count.get() as usize;
        // SAFETY: Permutation vectors and orientation vectors are shuffled
        // around, based on code from twsearch [1]. Testing has shown this is
        // sound.
        // [1] https://github.com/cubing/twsearch
        if orientation_count == 1.try_into().unwrap() {
            for i in 0..piece_count {
                let base_i = base + i;
                unsafe {
                    let pos = *a.get_unchecked(base + *b.get_unchecked(base_i) as usize);
                    *orbit_states_mut.get_unchecked_mut(base_i) = pos;
                    *orbit_states_mut.get_unchecked_mut(base_i + piece_count) = 0;
                }
            }
        } else {
            for i in 0..piece_count {
                let base_i = base + i;
                unsafe {
                    let pos = a.get_unchecked(base + *b.get_unchecked(base_i) as usize);
                    let a_ori =
                        a.get_unchecked(base + *b.get_unchecked(base_i) as usize + piece_count);
                    let b_ori = b.get_unchecked(base_i + piece_count);
                    *orbit_states_mut.get_unchecked_mut(base_i) = *pos;
                    *orbit_states_mut.get_unchecked_mut(base_i + piece_count) =
                        (*a_ori + *b_ori) % orientation_count;
                }
            }
        }
        base += piece_count * 2;
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
        let piece_count = piece_count.get() as usize;
        // SAFETY: Permutation vectors and orientation vectors are shuffled
        // around, based on code from twsearch [1]. Testing has shown this is
        // sound.
        // [1] https://github.com/cubing/twsearch
        if orientation_count == 1.try_into().unwrap() {
            for i in 0..piece_count {
                let base_i = base + i;
                unsafe {
                    *orbit_states_mut.get_unchecked_mut(base + a[base_i] as usize) = i as u8;
                    *orbit_states_mut.get_unchecked_mut(base + a[base_i] as usize + piece_count) =
                        0;
                }
            }
        } else {
            for i in 0..piece_count {
                let base_i = base + i;
                unsafe {
                    *orbit_states_mut.get_unchecked_mut(base + a[base_i] as usize) = i as u8;
                    *orbit_states_mut.get_unchecked_mut(base + a[base_i] as usize + piece_count) =
                        (orientation_count.get() - a[base_i + piece_count]) % orientation_count;
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
    for (
        &OrbitDef {
            piece_count,
            orientation_count,
        },
        partition,
    ) in sorted_orbit_defs.iter().zip(sorted_cycle_type.iter())
    {
        multi_bv.fill(0);
        let mut covered_cycles_count = 0_u8;
        let piece_count = piece_count.get() as usize;
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
            let mut piece = unsafe { *orbit_states.get_unchecked(base + i) } as usize;
            // SAFETY: sorted_orbit_defs guarantees that base (the orbit state
            // base pointer) + i + piece (an orientation vector element) is valid
            let mut orientation_sum =
                unsafe { *orbit_states.get_unchecked(base + piece + piece_count) };

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
                    piece = *orbit_states.get_unchecked(base + piece) as usize;
                }
                // SAFETY: sorted_orbit_defs guarantees that base (the orbit
                // state base pointer) + piece + piece_count (an orientation
                // vector element) is valid
                unsafe {
                    orientation_sum += *orbit_states.get_unchecked(base + piece + piece_count);
                }
            }

            let actual_orients = orientation_sum % orientation_count != 0;
            if actual_cycle_length == 1 && !actual_orients {
                continue;
            }
            let Some(valid_cycle_index) = partition.iter().enumerate().position(
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
            if covered_cycles_count > partition.len() as u8 {
                return false;
            }
        }
        if covered_cycles_count != partition.len() as u8 {
            return false;
        }
        base += piece_count * 2;
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
    let base = orbit_base_slice * piece_count * 2;
    let (permutation, orientation) = orbit_states.split_at(base + piece_count);
    (
        &permutation[base..base + piece_count],
        &orientation[base..base + piece_count],
    )
}

impl HeapPuzzle {
    /// Utility function for testing. Not optimized.
    pub fn cycle_type(
        &self,
        sorted_orbit_defs: &[OrbitDef],
        multi_bv: &mut [u8],
    ) -> Vec<Vec<(NonZeroU8, bool)>> {
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
                        .push((NonZeroU8::new(actual_cycle_length).unwrap(), actual_orients))
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
