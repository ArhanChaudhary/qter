#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::float_cmp)]

use std::{
    collections::{BTreeSet, HashMap},
    mem,
    num::NonZeroU16,
    sync::{Arc, LazyLock, OnceLock},
};

use edge_cloud::EdgeCloud;
use internment::ArcIntern;
use itertools::Itertools;
use knife::{CutSurface, do_cut};
use ksolve::{KSolve, KSolveMove, KSolveSet};
use num::{Matrix, Num, Vector, rotate_to, rotation_about};
use qter_core::{
    Span,
    architectures::{Permutation, PermutationGroup},
    union_find::UnionFind,
};
use thiserror::Error;

mod edge_cloud;
pub mod knife;
pub mod ksolve;
pub mod num;
pub mod shapes;

// Note... X is left to right, Y is down to up, and Z is forwards to backwards
// The coordinate system is right-handed

type PuzzleDescriptionString<'a> = &'a str;

#[derive(Error, Debug)]
pub enum PuzzleGeometryError {
    #[error("The vertices of the face are not coplanar: {0:?}")]
    FaceNotCoplanar(Face),
    #[error("The face forms a line or a point rather than a plane, or has collinear edges: {0:?}")]
    FaceIsDegenerate(Face),
    #[error(
        "A cut surface has cyclical structure and cannot be cut. Consider re-ordering the cut surfaces. Cut: {0}; Face: {1:?}"
    )]
    CyclicalCutSurface(String, Face),
    #[error("The slice {0} does not have any rotational symmetry")]
    PuzzleLacksSymmetry(ArcIntern<str>),
}

static DEG_180: LazyLock<Vector<2>> = LazyLock::new(|| Vector::new([[-1, 0]]));
static DEG_120: LazyLock<Vector<2>> = LazyLock::new(|| {
    Vector::new([[
        Num::from(-1) / Num::from(2),
        Num::from(1) / Num::from(2) * Num::from(3).sqrt(),
    ]])
});
static DEG_90: LazyLock<Vector<2>> = LazyLock::new(|| Vector::new([[0, 1]]));
static DEG_72: LazyLock<Vector<2>> = LazyLock::new(|| {
    let fourth = Num::from(1) / Num::from(4);
    Vector::new([[
        Num::from(5).sqrt() / Num::from(4) - fourth.clone(),
        (Num::from(2) * Num::from(5).sqrt() + Num::from(10)).sqrt() * fourth,
    ]])
});

#[derive(Clone, Debug)]
pub struct Point(Vector<3>);

#[derive(Clone, Debug)]
pub struct Face {
    pub points: Vec<Point>,
    pub color: ArcIntern<str>,
}

impl Face {
    fn is_valid(&self) -> Result<(), PuzzleGeometryError> {
        // TEST DEGENERACY

        if self.points.len() <= 2 {
            return Err(PuzzleGeometryError::FaceIsDegenerate(self.to_owned()));
        }

        if self
            .points
            .iter()
            .circular_tuple_windows()
            .any(|(a, b, c)| {
                let line = (b.0.clone() - a.0.clone()).normalize();
                // Projection matrix onto the line spanned by the first two points
                let line_proj = &line * &line.clone().transpose();

                (&line_proj * &(c.0.clone() - a.0.clone())) == (c.0.clone() - a.0.clone())
            })
        {
            return Err(PuzzleGeometryError::FaceIsDegenerate(self.to_owned()));
        }

        // TEST COPLANAR

        let FaceSubspaceInfo {
            make_3d,
            make_2d,
            offset,
        } = self.subspace_info();

        // Project points into the subspace
        let plane_proj = &make_3d * &make_2d;

        for point in self.points.iter().skip(3) {
            let offsetted = point.0.clone() - offset.clone();
            if &plane_proj * &offsetted != offsetted {
                return Err(PuzzleGeometryError::FaceNotCoplanar(self.to_owned()));
            }
        }

        Ok(())
    }

    fn transformed(&self, matrix: &Matrix<3, 3>) -> Self {
        Self {
            points: self
                .points
                .iter()
                .map(|point| Point(matrix * &point.0))
                .collect(),
            color: ArcIntern::clone(&self.color),
        }
    }

    fn edges(&self) -> impl Iterator<Item = (Vector<3>, Vector<3>)> {
        self.points
            .iter()
            .cycle()
            .tuple_windows()
            .take(self.points.len())
            .map(|(a, b)| (a.0.clone(), b.0.clone()))
    }

    fn edge_cloud(&self) -> EdgeCloud {
        EdgeCloud::new(self.edges().collect())
    }

    #[allow(dead_code)] // This is a false positive???
    fn epsilon_eq(&self, other: &Face) -> bool {
        self.edge_cloud().epsilon_eq(&other.edge_cloud())
    }

    /// Returns a pair of matrices where the first matrix projects a 2D vector into the 3D subspace spanned by this face, and the second computes the projection of a 3D vector into the 2D subspace.
    ///
    /// Also returns an origin vector to capture the translation of the face with respect to ⟨0, 0, 0⟩.
    fn subspace_info(&self) -> FaceSubspaceInfo {
        let offset = self.points[0].0.clone();

        // These two vectors define a 3D subspace that all points in the face should lie in
        let basis1 = self.points[1].0.clone() - offset.clone();
        let basis2 = self.points[2].0.clone() - offset.clone();

        // Transforms a 2D space into the 3D subspace
        // Make it orthogonal because that's nice to have
        let make_3d = Matrix::new([basis1.into_inner(), basis2.into_inner()]).mk_orthonormal();
        // Project points in 3D space into the subspace and into the 2D space
        // The transpose is the pseudo-inverse because `make_3d` is orthonormal and has full column rank
        let make_2d = make_3d.clone().transpose();

        FaceSubspaceInfo {
            make_3d,
            make_2d,
            offset,
        }
    }
}

/// Encodes the information about the plane on which a face lies.
#[derive(Clone, Debug)]
pub struct FaceSubspaceInfo {
    /// A matrix that converts a 2D vector to a 3D one in the subspace parallel to the face. To get a point on the face's plane, add `offset`.
    pub make_3d: Matrix<3, 2>,
    /// Projects a 3D vector into the subspace parallel to the face. Given a point on the face's plane, subtract `offset` first.
    pub make_2d: Matrix<2, 3>,
    /// The offset of the face from the origin. Subspaces must always include the origin due to how subspaces work mathematically so when projecting in/out, it is necessary to take the offset into account.
    pub offset: Vector<3>,
}

#[derive(Clone, Debug)]
pub struct Polyhedron(pub Vec<Face>);

#[derive(Clone, Debug)]
pub struct PuzzleGeometryDefinition {
    pub polyhedron: Polyhedron,
    pub cut_surfaces: Vec<Arc<dyn CutSurface>>,
    pub definition: Span,
}

#[derive(Clone, Debug)]
pub struct PuzzleGeometry {
    stickers: Vec<(Face, Vec<ArcIntern<str>>)>,
    turns: HashMap<ArcIntern<str>, (Vector<3>, Matrix<3, 3>, usize)>,
    definition: Span,
    perm_group: OnceLock<(Arc<PermutationGroup>, BTreeSet<usize>)>,
    non_fixed_stickers: OnceLock<Vec<(Face, Vec<ArcIntern<str>>)>>,
    ksolve: OnceLock<Arc<KSolve>>,
}

impl PuzzleGeometry {
    /// Get the puzzle as a permutation group over facelets
    fn permutation_group(&self) -> Arc<PermutationGroup> {
        Arc::clone(&self.calc_permutation_group().0)
    }

    fn calc_permutation_group(&self) -> &(Arc<PermutationGroup>, BTreeSet<usize>) {
        self.perm_group.get_or_init(|| {
            let clouds = self.stickers()
                .iter()
                .map(|v| v.0.edge_cloud())
                .collect::<Vec<_>>();

            let mut base_generators = Vec::new();

            for (name, turn) in &self.turns {
                let mut mapping = Vec::new();

                for sticker in self.stickers() {
                    if !sticker.1.contains(name) {
                        mapping.push(mapping.len());
                        continue;
                    }

                    let mut face = sticker.0.clone();
                    for point in &mut face.points {
                        *point = Point(&turn.1 * &(point.0.clone() - turn.0.clone()) + turn.0.clone());
                    }

                    let cloud = face.edge_cloud();

                    let (spot, _) = clouds
                        .iter()
                        .find_position(|test_cloud| cloud.epsilon_eq(test_cloud)).expect("We already verified this turn to work when creating the PuzzleGeometry instance");

                    mapping.push(spot);
                }

                base_generators.push((name, mapping, turn.2));
            }

            let to_skip = (0..self.stickers().len()).filter(|i| base_generators.iter().all(|(_, mapping, _)| mapping[*i] == *i)).collect::<BTreeSet<_>>();

            let mut generators = HashMap::new();

            for (name, mapping, symm) in base_generators {
                let base = Permutation::from_mapping(mapping.into_iter().enumerate().filter(|(i, _)| !to_skip.contains(i)).map(|(_, v)| v - to_skip.range(0..v).count()).collect());
                let mut current = base.clone();

                let names = turn_names(name, symm);

                for name in names {
                    generators.insert(name, current.clone());
                    current.compose_into(&base);
                }
            }

            (Arc::new(PermutationGroup::new(
                self.stickers()
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !to_skip.contains(i))
                    .map(|(_, v)| ArcIntern::clone(&v.0.color))
                    .collect(),
                generators,
                self.definition.clone(),
            )), to_skip)
        })
    }

    #[must_use]
    pub fn stickers(&self) -> &[(Face, Vec<ArcIntern<str>>)] {
        &self.stickers
    }

    pub fn non_fixed_stickers(&self) -> &[(Face, Vec<ArcIntern<str>>)] {
        self.non_fixed_stickers.get_or_init(|| {
            let (_, fixed) = self.calc_permutation_group();

            self.stickers
                .iter()
                .enumerate()
                .filter(|(i, _)| !fixed.contains(i))
                .map(|(_, v)| v.clone())
                .collect()
        })
    }

    /// Get the puzzle in its `KSolve` representation
    ///
    /// # Panics
    ///
    /// May panic if calculated numbers fall outside of the bit width of the fields of `KSolve`
    #[must_use]
    pub fn ksolve(&self) -> Arc<KSolve> {
        // Note: the KSolve permutation vector is **1-indexed**. See the test
        // cases for examples. It also exposes `zero_indexed_transformation` as
        // a convenience method.
        Arc::clone(self.ksolve.get_or_init(|| {
            let group = self.permutation_group();

            let mut sticker_orbits = UnionFind::<()>::new(group.facelet_count());

            for (_, generator) in group.generators() {
                for (a, b) in generator.mapping().iter().enumerate() {
                    sticker_orbits.union(a, *b, ());
                }
            }

            let mut pieces: HashMap<Vec<ArcIntern<str>>, Vec<usize>> = HashMap::new();

            for (sticker, (_, regions)) in self.non_fixed_stickers().iter().enumerate() {
                pieces
                    .entry(regions.iter().sorted_unstable().cloned().collect())
                    .or_default()
                    .push(sticker);
            }

            let mut orbits: Vec<Vec<Vec<usize>>> = Vec::new();

            'next_piece: for (_, piece) in pieces {
                let orbit_rep = sticker_orbits.find(piece[0]).root_idx();
                for maybe_orbit in &mut orbits {
                    if maybe_orbit[0].len() != piece.len() {
                        continue;
                    }

                    for facelet in &maybe_orbit[0] {
                        if sticker_orbits.find(*facelet).root_idx() == orbit_rep {
                            maybe_orbit.push(piece);
                            continue 'next_piece;
                        }
                    }
                }

                orbits.push(vec![piece]);
            }

            let mut sets: Vec<KSolveSet> = Vec::new();

            // Stores a tuple of (orientation number of sticker, piece index)
            // The first sticker in the list for each piece is considered the signature facelet. Plausibly we could have a better method for that but WHO CARES LOLOLOL (TODO)
            let mut facelet_orientation_numbers = vec![(0, 0); group.facelet_count()];

            for (i, orbit) in orbits.iter().enumerate() {
                let mut amt_in_same_orbit = 1;
                let first_rep = sticker_orbits.find(orbit[0][0]).root_idx();
                for sticker in orbit[0].iter().skip(1) {
                    if sticker_orbits.find(*sticker).root_idx() == first_rep {
                        amt_in_same_orbit += 1;
                    }
                }

                for (idx, piece) in orbit.iter().enumerate() {
                    let mut reps_to_count = HashMap::new();

                    for i in piece {
                        let rep = sticker_orbits.find(*i).root_idx();
                        let value = reps_to_count.entry(rep).or_insert(0);
                        facelet_orientation_numbers[*i] = (*value, idx);
                        *value += 1;
                    }
                }

                // TODO: Reasonable names?

                sets.push(KSolveSet {
                    name: i.to_string(),
                    piece_count: u16::try_from(orbit.len()).unwrap().try_into().unwrap(),
                    orientation_count: (u8::try_from(amt_in_same_orbit))
                        .unwrap()
                        .try_into()
                        .unwrap(),
                });
            }

            println!("{facelet_orientation_numbers:?}");

            let mut moves: Vec<KSolveMove> = Vec::new();

            for (name, perm) in group.generators() {
                let mut transformation = Vec::new();

                for orbit in &orbits {
                    let mut this_orbit_transform = Vec::new();

                    for piece in orbit {
                        let first_one_goes_to = perm.mapping()[piece[0]];
                        let (extra_orientation, piece_goes_to) =
                            facelet_orientation_numbers[first_one_goes_to];

                        this_orbit_transform.push((
                            NonZeroU16::try_from(u16::try_from(piece_goes_to + 1).unwrap())
                                .unwrap(),
                            u8::try_from(extra_orientation).unwrap(),
                        ));
                    }

                    transformation.push(this_orbit_transform);
                }

                moves.push(KSolveMove {
                    transformation,
                    name: name.to_string(),
                });
            }

            Arc::new(KSolve {
                name: self.definition.to_string(),
                sets,
                moves,
                symmetries: Vec::new(),
            })
        }))
    }
}

impl PuzzleGeometryDefinition {
    /// Consume a `PuzzleGeometryDefinition` and return a `PuzzleGeometry`
    ///
    /// # Errors
    ///
    /// If the validity of the faces is not satisfied, or if the puzzle does
    /// not have the expected symmetries, this function will return an error.
    #[expect(clippy::missing_panics_doc)]
    pub fn geometry(self) -> Result<PuzzleGeometry, PuzzleGeometryError> {
        let mut faces: Vec<(Face, Vec<ArcIntern<str>>)> = vec![];
        for face in self.polyhedron.0 {
            face.is_valid()?;
            faces.push((face, vec![]));
        }

        for cut_surface in &self.cut_surfaces {
            let mut new_faces = Vec::new();

            for (face, name_components) in faces {
                new_faces.extend(do_cut(&**cut_surface, &face)?.into_iter().map(
                    move |(new_face, name_component)| {
                        let mut name_components = name_components.clone();
                        if let Some(component) = name_component {
                            name_components.push(component);
                        }
                        (new_face, name_components)
                    },
                ));
            }

            faces = new_faces;
        }

        let stickers = faces;

        let mut turns = HashMap::new();
        let names = stickers.iter().flat_map(|v| v.1.iter()).unique();

        for name in names {
            let stickers = stickers
                .iter()
                .filter(|v| v.1.contains(name))
                .map(|(face, centroid)| (face, centroid.clone()))
                .collect_vec();

            // The center of mass must be preserved over rotations therefore any axis of symmetry must pass through it.
            let center_of_mass = stickers
                .iter()
                .flat_map(|v| &v.0.points)
                .map(|v| v.0.clone())
                .sum::<Vector<3>>()
                / &Num::from(stickers.len());

            let mut edges = stickers.iter().flat_map(|v| v.0.edges()).collect_vec();

            for edge in &mut edges {
                edge.0 -= center_of_mass.clone();
                edge.1 -= center_of_mass.clone();
            }

            // Narrow down the edges that could potentially map to each other so that we don't have to try all of them
            let mut edge_classifications: Vec<((Num, Num), Vec<(Matrix<3, 1>, Matrix<3, 1>)>)> =
                Vec::new();

            'next_edge: for edge in &edges {
                let mut a = edge.0.clone().norm_squared();
                let mut b = edge.1.clone().norm_squared();
                if a > b {
                    mem::swap(&mut a, &mut b);
                }

                for ((maybe_a, maybe_b), list) in &mut edge_classifications {
                    if a == *maybe_a && b == *maybe_b {
                        list.push(edge.clone());
                        continue 'next_edge;
                    }
                }

                edge_classifications.push(((a, b), vec![edge.clone()]));
            }

            let edges_that_might_map_together = edge_classifications
                .into_iter()
                .min_by_key(|v| v.1.len())
                .unwrap()
                .1;

            let from = Matrix::new([
                edges_that_might_map_together[0].0.clone().into_inner(),
                edges_that_might_map_together[0].1.clone().into_inner(),
            ]);

            let matrices = edges_that_might_map_together
                .into_iter()
                .flat_map(|(a, b)| [(a.clone(), b.clone()), (b, a)])
                .skip(1)
                .map(|v| {
                    let to = Matrix::new([v.0.into_inner(), v.1.into_inner()]);
                    rotate_to(from.clone(), to)
                });

            let cloud = EdgeCloud::new(edges);

            match matrices
                .filter_map(|matrix| {
                    cloud
                        .clone()
                        .try_symmetry(&matrix)
                        .map(|degree| (matrix, degree))
                })
                .max_by_key(|v| v.1)
            {
                None | Some((_, 1)) => {
                    return Err(PuzzleGeometryError::PuzzleLacksSymmetry(name.clone()));
                }
                Some((matrix, degree)) => {
                    turns.insert(name.clone(), (center_of_mass, matrix, degree));
                }
            }
        }

        Ok(PuzzleGeometry {
            stickers,
            turns,
            definition: self.definition,
            perm_group: OnceLock::new(),
            ksolve: OnceLock::new(),
            non_fixed_stickers: OnceLock::new(),
        })
    }
}

fn turn_names(base_name: &ArcIntern<str>, symm: usize) -> Vec<ArcIntern<str>> {
    let mut names_begin = Vec::new();
    let mut names_end = Vec::new();

    let mut i = 1;

    while names_begin.len() + names_end.len() < symm - 1 {
        if names_begin.len() == names_end.len() {
            if i == 1 {
                names_begin.push(ArcIntern::clone(base_name));
            } else {
                names_begin.push(ArcIntern::from(format!("{base_name}{i}")));
            }
        } else {
            if i == 1 {
                names_end.push(ArcIntern::from(format!("{base_name}'")));
            } else {
                names_end.push(ArcIntern::from(format!("{base_name}{i}'")));
            }

            i += 1;
        }
    }

    names_begin.extend(names_end.into_iter().rev());

    names_begin
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        Face, Point, PuzzleGeometryDefinition, PuzzleGeometryError,
        knife::PlaneCut,
        num::{Num, Vector},
        shapes::{CUBE, TETRAHEDRON},
        turn_names,
    };
    use internment::ArcIntern;
    use qter_core::{Int, Span, U, schreier_sims::StabilizerChain};

    #[test]
    fn test_turn_names() {
        assert_eq!(
            turn_names(&ArcIntern::from("R"), 4),
            [
                ArcIntern::from("R"),
                ArcIntern::from("R2"),
                ArcIntern::from("R'")
            ]
        );
        assert_eq!(
            turn_names(&ArcIntern::from("U"), 5),
            [
                ArcIntern::from("U"),
                ArcIntern::from("U2"),
                ArcIntern::from("U2'"),
                ArcIntern::from("U'")
            ]
        );
    }

    #[test]
    fn degeneracy() {
        let valid = Face {
            points: vec![Point(Vector::new([[1, 2, 3]]))],
            color: ArcIntern::from("aliceblue"),
        }
        .is_valid();
        assert!(matches!(
            valid,
            Err(PuzzleGeometryError::FaceIsDegenerate(_))
        ));

        let valid = Face {
            points: vec![
                Point(Vector::new([[1, 2, 3]])),
                Point(Vector::new([[5, 4, 3]])),
            ],
            color: ArcIntern::from("oklch(1 2 3)"),
        }
        .is_valid();
        assert!(matches!(
            valid,
            Err(PuzzleGeometryError::FaceIsDegenerate(_))
        ));

        let valid = Face {
            points: vec![
                Point(Vector::new([[2, 2, 3]])),
                Point(Vector::new([[3, 4, 6]])),
                Point(Vector::new([[4, 6, 9]])),
            ],
            color: ArcIntern::from("fuschia"),
        }
        .is_valid();
        assert!(matches!(
            valid,
            Err(PuzzleGeometryError::FaceIsDegenerate(_))
        ));
    }

    #[test]
    fn not_coplanar() {
        let valid = Face {
            points: vec![
                Point(Vector::new([[2, 2, 3]])),
                Point(Vector::new([[3, 4, 6]])),
                Point(Vector::new([[4, 6, 11]])),
                Point(Vector::new([[6, 6, 11]])),
            ],
            color: ArcIntern::from("blue"),
        }
        .is_valid();

        assert!(matches!(
            valid,
            Err(PuzzleGeometryError::FaceNotCoplanar(_))
        ));

        let valid = Face {
            points: vec![
                Point(Vector::new([[1, 1, 1]])),
                Point(Vector::new([[1, 1, 0]])),
                Point(Vector::new([[1, 0, 0]])),
                Point(Vector::new([[1, 0, 1]])),
            ],
            color: ArcIntern::from("bruh"),
        }
        .is_valid();

        assert!(matches!(valid, Ok(())));
    }

    #[test]
    fn three_by_three() {
        let cube = PuzzleGeometryDefinition {
            polyhedron: CUBE.to_owned(),
            cut_surfaces: vec![
                Arc::from(PlaneCut {
                    spot: Vector::new_ratios([[(1, 3), (0, 1), (0, 1)]]),
                    normal: Vector::new([[1, 0, 0]]),
                    name: ArcIntern::from("R"),
                }),
                Arc::from(PlaneCut {
                    spot: Vector::new_ratios([[(-1, 3), (0, 1), (0, 1)]]),
                    normal: Vector::new([[-1, 0, 0]]),
                    name: ArcIntern::from("L"),
                }),
                Arc::from(PlaneCut {
                    spot: Vector::new_ratios([[(0, 1), (1, 3), (0, 1)]]),
                    normal: Vector::new([[0, 1, 0]]),
                    name: ArcIntern::from("U"),
                }),
                Arc::from(PlaneCut {
                    spot: Vector::new_ratios([[(0, 1), (-1, 3), (0, 1)]]),
                    normal: Vector::new([[0, -1, 0]]),
                    name: ArcIntern::from("D"),
                }),
                Arc::from(PlaneCut {
                    spot: Vector::new_ratios([[(0, 1), (0, 1), (1, 3)]]),
                    normal: Vector::new([[0, 0, 1]]),
                    name: ArcIntern::from("F"),
                }),
                Arc::from(PlaneCut {
                    spot: Vector::new_ratios([[(0, 1), (0, 1), (-1, 3)]]),
                    normal: Vector::new([[0, 0, -1]]),
                    name: ArcIntern::from("B"),
                }),
            ],
            definition: Span::new(ArcIntern::from("3x3"), 0, 3),
        };

        let geometry = cube.geometry().unwrap();
        assert_eq!(geometry.stickers().len(), 54);

        for turn in &geometry.turns {
            assert_eq!(turn.1.2, 4);
        }
        assert_eq!(geometry.turns.len(), 6);

        let group = geometry.permutation_group();
        assert_eq!(group.facelet_count(), 48);

        assert_eq!(
            StabilizerChain::new(&group).cardinality(),
            "43252003274489856000".parse::<Int<U>>().unwrap()
        );

        // panic!("{:#?}", geometry.ksolve());
    }

    #[test]
    fn pyraminx() {
        let up = TETRAHEDRON.0[0].points[0].clone().0;
        let down1 = TETRAHEDRON.0[3].points[0].clone().0;
        let down2 = TETRAHEDRON.0[3].points[1].clone().0;
        let down3 = TETRAHEDRON.0[3].points[2].clone().0;

        let pyraminx = PuzzleGeometryDefinition {
            polyhedron: TETRAHEDRON.to_owned(),
            cut_surfaces: vec![
                Arc::from(PlaneCut {
                    spot: up.clone().normalize() / &Num::from(3),
                    normal: up.clone(),
                    name: ArcIntern::from("A"),
                }),
                Arc::from(PlaneCut {
                    spot: down1.clone().normalize() / &Num::from(3),
                    normal: down1.clone(),
                    name: ArcIntern::from("B"),
                }),
                Arc::from(PlaneCut {
                    spot: down2.clone().normalize() / &Num::from(3),
                    normal: down2.clone(),
                    name: ArcIntern::from("C"),
                }),
                Arc::from(PlaneCut {
                    spot: down3.clone().normalize() / &Num::from(3),
                    normal: down3.clone(),
                    name: ArcIntern::from("D"),
                }),
                Arc::from(PlaneCut {
                    spot: (up.clone().normalize() / &Num::from(3)) * &Num::from(5),
                    normal: up.clone(),
                    name: ArcIntern::from("E"),
                }),
                Arc::from(PlaneCut {
                    spot: (down1.clone().normalize() / &Num::from(3)) * &Num::from(5),
                    normal: down1.clone(),
                    name: ArcIntern::from("F"),
                }),
                Arc::from(PlaneCut {
                    spot: (down2.clone().normalize() / &Num::from(3)) * &Num::from(5),
                    normal: down2.clone(),
                    name: ArcIntern::from("G"),
                }),
                Arc::from(PlaneCut {
                    spot: (down3.clone().normalize() / &Num::from(3)) * &Num::from(5),
                    normal: down3.clone(),
                    name: ArcIntern::from("H"),
                }),
            ],
            definition: Span::new(ArcIntern::from("pyraminx"), 0, 8),
        };

        let geometry = pyraminx.geometry().unwrap();
        assert_eq!(geometry.stickers().len(), 36);

        for turn in &geometry.turns {
            assert_eq!(turn.1.2, 3);
        }
        assert_eq!(geometry.turns.len(), 8);

        let group = geometry.permutation_group();
        assert_eq!(group.facelet_count(), 36);

        assert_eq!(
            StabilizerChain::new(&group).cardinality(),
            "75582720".parse::<Int<U>>().unwrap()
        );
    }
}
