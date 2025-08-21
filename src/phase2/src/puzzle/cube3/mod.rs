//! SIMD optimized implementations for 3x3 cubes

#[cfg(not(any(avx2, simd8and16)))]
pub type Cube3 = super::slice_puzzle::StackPuzzle<40>;

mod common {
    use crate::orbit_puzzle::cube3::Cube3Edges;
    use crate::orbit_puzzle::cubeN::CubeNCorners;
    use crate::orbit_puzzle::{OrbitPuzzleStateImplementor, SpecializedOrbitPuzzleState};
    use crate::puzzle::{
        AuxMem, AuxMemRefMut, BrandedOrbitDef, OrbitDef, OrbitIdentifier, PuzzleState,
        SortedCycleTypeRef, SortedOrbitDefsRef, TransformationsMeta, TransformationsMetaError,
        cube3,
    };
    use generativity::Id;
    use std::hash::Hash;
    use std::{fmt::Debug, num::NonZeroU8};

    /// An orbit identifier for 3x3 cubes.
    #[derive(Debug, Clone, Copy)]
    pub enum Cube3OrbitType {
        /// The corners orbit.
        Corners,
        /// The edges orbit.
        Edges,
    }

    pub use private::*;
    mod private {
        //! Private module to disallow explicit instantiation of
        //! `CornersTransformation` and `EdgesTransformation`.

        /// A valid transformation for the corners and edges of a 3x3 cube.
        pub struct CornersTransformation<'a>(&'a [(u8, u8); 8]);

        /// A valid transformation for the edges of a 3x3 cube.
        pub struct EdgesTransformation<'a>(&'a [(u8, u8); 12]);

        impl<'a> CornersTransformation<'a> {
            /// Create a new `CornersTransformation`
            ///
            /// # Safety
            ///
            /// The caller must ensure that `corners_transformation` is from a
            /// `TransformationMeta`,
            pub unsafe fn new_unchecked(corners_transformation: &'a [(u8, u8); 8]) -> Self {
                Self(corners_transformation)
            }

            /// Get the corners transformation as a slice.
            pub fn get(&self) -> &'a [(u8, u8); 8] {
                self.0
            }
        }

        impl<'a> EdgesTransformation<'a> {
            /// Create a new `EdgesTransformation`
            ///
            /// # Safety
            ///
            /// The caller must ensure that `edges_transformation` is from a
            /// `TransformationMeta`.
            pub unsafe fn new_unchecked(edges_transformation: &'a [(u8, u8); 12]) -> Self {
                Self(edges_transformation)
            }

            /// Get the edges transformation as a slice.
            pub fn get(&self) -> &'a [(u8, u8); 12] {
                self.0
            }
        }
    }

    /// The interface for a 3x3 cube puzzle state
    pub trait Cube3Interface: Clone + PartialEq + Debug + 'static {
        fn from_corner_and_edge_transformations(
            corners_transformation: CornersTransformation<'_>,
            edges_transformation: EdgesTransformation<'_>,
        ) -> Self;

        /// Compose `a` and `b` into self.
        fn replace_compose(&mut self, a: &Self, b: &Self);

        /// Inverse `a` into self.
        fn replace_inverse(&mut self, a: &Self);

        /// Check if the cube induces a sorted cycle type.
        fn induces_sorted_cycle_type(&self, sorted_cycle_type: SortedCycleTypeRef) -> bool;

        /// Convert an orbit of the cube state into a pair of (perm, ori) bytes.
        /// For implementation reasons that should ideally be abstracted away,
        /// we have to make the arrays length 16.
        fn orbit_bytes(&self, orbit_type: Cube3OrbitType) -> ([u8; 16], [u8; 16]);

        /// Exact hasher for an orbit. Note that this is different from a
        /// "hash", which in Rust terminology is something that implements Hash
        fn exact_hasher_orbit(&self, orbit_type: Cube3OrbitType) -> u64;

        /// Approximate hash for an orbit
        fn approximate_hash_orbit(&self, orbit_type: Cube3OrbitType) -> impl Hash;
    }

    /// The expected sorted orbit definition for 3x3 puzzles.
    pub const CUBE_3_SORTED_ORBIT_DEFS: [OrbitDef; 2] = [
        OrbitDef {
            piece_count: NonZeroU8::new(8).unwrap(),
            orientation_count: NonZeroU8::new(3).unwrap(),
        },
        OrbitDef {
            piece_count: NonZeroU8::new(12).unwrap(),
            orientation_count: NonZeroU8::new(2).unwrap(),
        },
    ];

    impl<'id> OrbitIdentifier<'id> for Cube3OrbitType {
        fn first_orbit_identifier(_branded_orbit_def: BrandedOrbitDef<'id>) -> Self {
            Cube3OrbitType::Corners
        }

        fn next_orbit_identifier(self, _branded_orbit_def: BrandedOrbitDef<'id>) -> Self {
            match self {
                Cube3OrbitType::Corners => Cube3OrbitType::Edges,
                Cube3OrbitType::Edges => panic!("No next orbit identifier for Cube3"),
            }
        }

        fn orbit_def(&self) -> OrbitDef {
            match self {
                Cube3OrbitType::Corners => CUBE_3_SORTED_ORBIT_DEFS[0],
                Cube3OrbitType::Edges => CUBE_3_SORTED_ORBIT_DEFS[1],
            }
        }
    }

    macro_rules! impl_puzzle_state_for_cube3 {
        ($($cube3:path),* $(,)?) => {$(
            impl<'id> PuzzleState<'id> for $cube3 {
                type OrbitBytesBuf<'a> = [u8; 16];
                type OrbitIdentifier = Cube3OrbitType;

                fn new_aux_mem(sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>) -> AuxMem<'id> {
                    AuxMem::new(None, sorted_orbit_defs.id())
                }

                fn try_from_transformations_meta(
                    transformations_meta: TransformationsMeta<'id, '_>,
                    _id: Id<'id>,
                ) -> Result<Self, TransformationsMetaError> {
                    let sorted_orbit_defs = transformations_meta.sorted_orbit_defs().inner;
                    if sorted_orbit_defs == CUBE_3_SORTED_ORBIT_DEFS {
                        let sorted_transformations = transformations_meta.sorted_transformations();
                        // SAFETY: `TransformationMeta` guarantees that the sorted
                        // transformations have the same length as its sorted orbit
                        // definitions, which we just proved to be 2.
                        let sorted_transformations: &[Vec<(u8, u8)>; 2] =
                            unsafe { sorted_transformations.try_into().unwrap_unchecked() };
                        // SAFETY: `TransformationMeta` guarantees that the first orbit
                        // corresponds to the first sorted orbit definition, which we
                        // have just proven to be the corners orbit.
                        let corners_transformation = unsafe {
                            sorted_transformations[0]
                                .as_slice()
                                .try_into()
                                .unwrap_unchecked()
                        };
                        // SAFETY: `TransformationMeta` guarantees that the second orbit
                        // corresponds to the second sorted orbit definition, which we
                        // have just proven to be the edges orbit.
                        let edges_transformation = unsafe {
                            sorted_transformations[1]
                                .as_slice()
                                .try_into()
                                .unwrap_unchecked()
                        };
                        Ok(Self::from_corner_and_edge_transformations(
                            // SAFETY: `corner_transformation` is from a
                            // `TransformationMeta`
                            unsafe { CornersTransformation::new_unchecked(corners_transformation) },
                            // SAFETY: `edges_transformation` is from a
                            // `TransformationMeta`
                            unsafe { EdgesTransformation::new_unchecked(edges_transformation) },
                        ))
                    } else {
                        Err(TransformationsMetaError::InvalidOrbitDefs {
                            expected: CUBE_3_SORTED_ORBIT_DEFS.to_vec(),
                            actual: sorted_orbit_defs.to_vec(),
                        })
                    }
                }

                fn replace_compose(
                    &mut self,
                    a: &Self,
                    b: &Self,
                    _sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
                ) {
                    Cube3Interface::replace_compose(self, a, b);
                }

                fn replace_inverse(&mut self, a: &Self, _sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>) {
                    Cube3Interface::replace_inverse(self, a);
                }

                fn induces_sorted_cycle_type(
                    &self,
                    sorted_cycle_type: SortedCycleTypeRef<'id, '_>,
                    _sorted_orbit_defs: SortedOrbitDefsRef<'id, '_>,
                    _aux_mem: AuxMemRefMut<'id, '_>,
                ) -> bool {
                    Cube3Interface::induces_sorted_cycle_type(self, sorted_cycle_type)
                }

                fn orbit_bytes(&self, orbit_identifier: Cube3OrbitType) -> ([u8; 16], [u8; 16]) {
                    Cube3Interface::orbit_bytes(self, orbit_identifier)
                }

                fn exact_hasher_orbit(&self, orbit_identifier: Cube3OrbitType) -> u64 {
                    Cube3Interface::exact_hasher_orbit(self, orbit_identifier)
                }

                fn approximate_hash_orbit(&self, orbit_identifier: Cube3OrbitType) -> impl Hash {
                    Cube3Interface::approximate_hash_orbit(self, orbit_identifier)
                }

                fn pick_orbit_puzzle(
                    orbit_identifier: Self::OrbitIdentifier,
                ) -> OrbitPuzzleStateImplementor {
                    match orbit_identifier {
                        Cube3OrbitType::Corners => unsafe {
                            CubeNCorners::new_solved_state(orbit_identifier.orbit_def()).into()
                        },
                        Cube3OrbitType::Edges => unsafe {
                            Cube3Edges::new_solved_state(orbit_identifier.orbit_def()).into()
                        },
                    }
                }
            }
        )*}
    }

    impl_puzzle_state_for_cube3!(
        cube3::simd8and16::UncompressedCube3,
        cube3::simd8and16::Cube3,
        cube3::avx2::Cube3
    );
}

pub(in crate::puzzle) mod avx2;
pub(in crate::puzzle) mod simd8and16;

#[cfg(avx2)]
pub use avx2::Cube3;

#[cfg(all(not(avx2), simd8and16))]
pub use simd8and16::Cube3;

// pub struct StackEvenCubeSimd<const S_24S: usize> {
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32; S_24S],
// }

// pub struct HeapEvenCubeSimd {
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32],
// }

// pub struct StackOddCubeSimd<const S_24S: usize> {
//     ep: u8x16,
//     eo: u8x16,
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32; S_24S],
// }

// pub struct HeapOddCubeSimd {
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32],
// }
