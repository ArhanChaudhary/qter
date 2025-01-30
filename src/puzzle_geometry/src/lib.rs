mod puzzles;
use std::{cmp::Ordering, collections::HashMap, mem};

use internment::ArcIntern;
use itertools::Itertools;
use nalgebra::{Matrix2, Matrix3, Matrix3x2, Rotation3, Unit, Vector3};
pub use puzzles::*;

mod puzzle_geometry;
pub use puzzle_geometry::*;
use qter_core::architectures::PermutationGroup;
use thiserror::Error;

mod defaults;
mod options;

// Margin of error to consider points "equal"
const E: f64 = 1e-9;

type PuzzleDescriptionString<'a> = &'a str;

#[derive(Error, Debug)]
pub enum Error {
    #[error("The vertices of the face are not coplanar: {0:?}")]
    FaceNotCoplanar(Face),
    #[error("The face is not convex: {0:?}")]
    FaceNotConvex(Face),
    #[error("The face forms a line or a point rather than a plane: {0:?}")]
    FaceIsDegenerate(Face),
    #[error(
        "The puzzle does not have {1}-fold rotational symmetry as expected by the cut line: {0:?}"
    )]
    PuzzleLacksExpectedSymmetry(Vector3<f64>, u8),
    #[error("The puzzle does not have any rotational symmetry along the cut line: {0:?}")]
    PuzzleLacksSymmetry(Vector3<f64>),
}

#[derive(Clone, Copy, Debug)]
pub struct Point(Vector3<f64>);

#[derive(Clone, Debug)]
pub struct CutAxisNames<'a> {
    /// The names for slices forward of the cut plane
    pub forward_name: &'a str,
    /// The names for slices backward of the cut plane
    pub backward_name: &'a str,
    /// The name of the odd slice in the middle
    pub middle_name: &'a str,
}

#[derive(Clone, Debug)]
pub struct CutAxis<'a> {
    pub names: CutAxisNames<'a>,
    /// The expected degree of symmetry of the cut. If this is `None`, the symmetry will be auto detected.
    /// Otherwise, this symmetry will be verified and used.
    pub expected_symmetry: Option<u8>,
    /// Direction is normal to the cut plane
    pub normal: Unit<Vector3<f64>>,
    /// The distances away from the origin for all of the slices
    pub distances: Vec<f64>,
}

fn rotation_of_degree(axis: Vector3<f64>, symm: u8) -> Matrix3<f64> {
    Rotation3::from_axis_angle(
        &Unit::new_normalize(axis),
        core::f64::consts::TAU / (symm as f64),
    )
    .into()
}

#[derive(Clone, Debug)]
pub struct Face(pub Vec<Point>);

impl Face {
    /// Rotate the face around the origin with the given axis and symmetry
    pub fn rotated(mut self, axis: Vector3<f64>, symmetry: u8) -> Face {
        assert_ne!(symmetry, 0);

        let rotation = rotation_of_degree(axis, symmetry);

        for point in &mut self.0 {
            point.0 = rotation * point.0;
        }

        self
    }

    fn is_valid(&self) -> Result<(), Error> {
        // TEST DEGENERACY

        if self.0.len() <= 2 {
            return Err(Error::FaceIsDegenerate(self.to_owned()));
        }

        let origin = self.0[0].0;

        let line = (self.0[1].0 - origin).normalize();
        // Projection matrix onto the line spanned by the first two points
        let line_proj = line * line.transpose();

        for point in self.0.iter().skip(2) {
            let offsetted = point.0 - origin;
            if (line_proj * offsetted).metric_distance(&offsetted) < E {
                return Err(Error::FaceIsDegenerate(self.to_owned()));
            }
        }

        // TEST COPLANAR

        // These two vectors define a 3D subspace that all points in the face should lie in
        let basis1 = self.0[1].0 - origin;
        let basis2 = self.0[2].0 - origin;

        // Transform a 2D space into the 3D subspace
        let make_3d = Matrix3x2::from_columns(&[basis1, basis2]);
        // Project points in 3D space into the subspace and into the 2D space
        let make_2d = make_3d.pseudo_inverse(E).unwrap();
        // Project points into the subspace
        let plane_proj = make_3d * make_2d;

        for point in self.0.iter().skip(3) {
            let offsetted = point.0 - origin;
            if (plane_proj * offsetted).metric_distance(&offsetted) >= E {
                return Err(Error::FaceNotCoplanar(self.to_owned()));
            }
        }

        // TEST CONVEXITY

        // All pairs of three points must all be either be either left turns or right turns
        if !self
            .0
            .iter()
            .map(|point| make_2d * (point.0 - origin))
            .cycle()
            .tuple_windows()
            .take(self.0.len())
            .filter_map(|(a, b, c)| {
                let v = Matrix2::from_columns(&[a - b, c - b])
                    .determinant()
                    .signum();

                if v == 0. {
                    None
                } else {
                    Some(v)
                }
            })
            .all_equal()
        {
            return Err(Error::FaceNotConvex(self.to_owned()));
        }

        Ok(())
    }

    fn edge_cloud(&self) -> Vec<(Vector3<f64>, Vector3<f64>)> {
        let mut cloud = Vec::new();

        for (vertex1, vertex2) in self.0.iter().cycle().tuple_windows().take(self.0.len()) {
            cloud.push((vertex1.0, vertex2.0));
        }

        cloud
    }

    fn epsilon_eq(&self, other: &Face) -> bool {
        edge_cloud_eq(&self.edge_cloud(), &other.edge_cloud())
    }
}

#[derive(Clone, Debug)]
pub struct Polyhedron(pub Vec<Face>);

#[derive(Clone, Debug)]
pub struct PuzzleDefinition<'a> {
    pub polyhedron: Polyhedron,
    pub cut_axes: Vec<CutAxis<'a>>,
}

#[derive(Clone, Debug)]
pub struct PuzzleGeometry {}

impl PuzzleGeometry {
    /// Get the puzzle as a permutation group over facelets
    pub fn permutation_group(&self) -> &PermutationGroup {
        todo!()
    }

    /// Get the puzzle as a permutation and orientation group over pieces
    pub fn piece_perm_and_ori_group(&self) -> &PiecePermAndOriGroup {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct PiecePermAndOriGroup {}

impl PiecePermAndOriGroup {
    /// For each type of piece, return a list of (amount of the piece type, orientation mod)
    pub fn pieces(&self) -> &[(usize, u8)] {
        todo!()
    }

    /// Get the set of available moves on the puzzle
    pub fn moves(&self) -> &HashMap<ArcIntern<str>, PuzzleState> {
        todo!()
    }

    /// Get the list of symmetries obeyed by the puzzle
    pub fn symmetries(&self) -> &[PuzzleState] {
        todo!()
    }
}

impl<'a> PuzzleDefinition<'a> {
    pub fn geometry(mut self) -> Result<PuzzleGeometry, Error> {
        for face in &self.polyhedron.0 {
            face.is_valid()?;
        }

        self.find_symmetries()?;

        Ok(PuzzleGeometry {})
    }

    fn find_symmetries(&mut self) -> Result<(), Error> {
        let cloud = self.edge_cloud();
        let mut into = cloud.to_owned();

        for cut in &mut self.cut_axes {
            match cut.expected_symmetry {
                Some(symm) => {
                    let matrix = rotation_of_degree(*cut.normal, symm);

                    if !try_symmetry(&cloud, &mut into, matrix) {
                        return Err(Error::PuzzleLacksExpectedSymmetry(*cut.normal, symm));
                    }
                }
                None => {
                    let mut min_symm = 1_u8;
                    let mut trying_symm = 1_u8;

                    loop {
                        trying_symm = match trying_symm.checked_add(min_symm) {
                            Some(new_symm) => new_symm,
                            None => break,
                        };

                        let matrix = rotation_of_degree(*cut.normal, trying_symm);

                        if try_symmetry(&cloud, &mut into, matrix) {
                            min_symm = trying_symm;
                        }
                    }

                    cut.expected_symmetry = Some(min_symm);
                }
            }

            if cut.expected_symmetry.unwrap() <= 1 {
                return Err(Error::PuzzleLacksSymmetry(*cut.normal));
            }
        }

        Ok(())
    }

    /// A sorted list of sorted points, used for structural equality
    fn edge_cloud(&self) -> Vec<(Vector3<f64>, Vector3<f64>)> {
        let mut cloud = Vec::new();

        // TODO: Handle these cuts separately, mixing them in with the edges won't work in general

        for cut_axis in &self.cut_axes {
            // Cuts must also have the correct symmetry, but they are automatically rotationally symmetric with themselves
            for cut in &cut_axis.distances {
                // Cannot make a cut at distance zero
                if *cut < E {
                    continue;
                }

                let cut_spot = *cut_axis.normal * *cut;
                cloud.push((cut_spot, -cut_spot));
            }

            // In the case of just a cut through the middle, this will still ensure symmetry
            cloud.push((*cut_axis.normal, -*cut_axis.normal));
        }

        for face in &self.polyhedron.0 {
            cloud.extend_from_slice(&face.edge_cloud());
        }

        sort_edge_cloud(&mut cloud);

        cloud
    }
}

fn try_symmetry(
    cloud: &[(Vector3<f64>, Vector3<f64>)],
    into: &mut [(Vector3<f64>, Vector3<f64>)],
    matrix: Matrix3<f64>,
) -> bool {
    into.copy_from_slice(cloud);

    for point in into.iter_mut().flat_map(|(a, b)| [a, b]) {
        *point = matrix * *point;
    }

    sort_edge_cloud(into);

    edge_cloud_eq(cloud, into)
}

fn sort_edge_cloud(cloud: &mut [(Vector3<f64>, Vector3<f64>)]) {
    for (a, b) in &mut *cloud {
        let ordering = a
            .iter()
            .zip(b.iter())
            .map(|(x1, x2)| {
                if (x1 - x2).abs() < E {
                    return Ordering::Equal;
                }

                x1.total_cmp(x2)
            })
            .find_or_last(|v| !matches!(v, Ordering::Equal))
            .unwrap();

        if matches!(ordering, Ordering::Greater) {
            mem::swap(a, b);
        }
    }

    cloud.sort_unstable_by(|(a1, b1), (a2, b2)| {
        a1.as_slice()
            .iter()
            .zip(a2.as_slice().iter())
            .chain(b1.as_slice().iter().zip(b2.as_slice().iter()))
            .map(|(x1, x2)| {
                if (x1 - x2).abs() < E {
                    return Ordering::Equal;
                }

                x1.total_cmp(x2)
            })
            .find_or_last(|v| !matches!(v, Ordering::Equal))
            .unwrap()
    });
}

fn edge_cloud_eq(
    cloud1: &[(Vector3<f64>, Vector3<f64>)],
    cloud2: &[(Vector3<f64>, Vector3<f64>)],
) -> bool {
    cloud1
        .iter()
        .zip(cloud2)
        .all(|((a1, b1), (a2, b2))| (a1.metric_distance(a2) < E) && (b1.metric_distance(b2) < E))
}

#[cfg(test)]
mod tests {
    use nalgebra::{Unit, Vector3};

    use crate::{CutAxis, CutAxisNames, Error, Face, Point, Polyhedron, PuzzleDefinition, CUBE};

    #[test]
    fn degeneracy() {
        let valid = Face(vec![Point(Vector3::new(1., 2., 3.))]).is_valid();
        assert!(matches!(valid, Err(Error::FaceIsDegenerate(_))));

        let valid = Face(vec![
            Point(Vector3::new(1., 2., 3.)),
            Point(Vector3::new(5., 4., 3.)),
        ])
        .is_valid();
        assert!(matches!(valid, Err(Error::FaceIsDegenerate(_))));

        let valid = Face(vec![
            Point(Vector3::new(2., 2., 3.)),
            Point(Vector3::new(3., 4., 6.)),
            Point(Vector3::new(4., 6., 9.)),
        ])
        .is_valid();
        assert!(matches!(valid, Err(Error::FaceIsDegenerate(_))));
    }

    #[test]
    fn not_coplanar() {
        let valid = Face(vec![
            Point(Vector3::new(2., 2., 3.)),
            Point(Vector3::new(3., 4., 6.)),
            Point(Vector3::new(4., 6., 11.)),
            Point(Vector3::new(6., 6., 11.)),
        ])
        .is_valid();

        assert!(matches!(valid, Err(Error::FaceNotCoplanar(_))));

        let valid = Face(vec![
            Point(Vector3::new(1., 1., 1.)),
            Point(Vector3::new(1., 1., 0.)),
            Point(Vector3::new(1., 0., 0.)),
            Point(Vector3::new(1., 0., 1.)),
        ])
        .is_valid();

        assert!(matches!(valid, Ok(())));
    }

    #[test]
    fn not_convex() {
        let valid = Face(vec![
            Point(Vector3::new(1., 1., 1.)),
            Point(Vector3::new(1., 0., 0.)),
            Point(Vector3::new(1., 1., 0.)),
            Point(Vector3::new(1., 0., 1.)),
        ])
        .is_valid();

        assert!(matches!(valid, Err(Error::FaceNotConvex(_))));

        let valid = Face(vec![
            Point(Vector3::new(1., 1., 1.)),
            Point(Vector3::new(1., 1., 0.)),
            Point(Vector3::new(1., 0.8, 0.8)),
            Point(Vector3::new(1., 0., 1.)),
        ])
        .is_valid();

        assert!(matches!(valid, Err(Error::FaceNotConvex(_))));

        // Verify left turns and right turns
        let valid = Face(vec![
            Point(Vector3::new(1., 0., 1.)),
            Point(Vector3::new(1., 0., 0.)),
            Point(Vector3::new(1., 1., 0.)),
            Point(Vector3::new(1., 1., 1.)),
        ])
        .is_valid();

        assert!(matches!(valid, Ok(())));
    }

    #[test]
    fn symmetries_simple() {
        let mut one_face = PuzzleDefinition {
            polyhedron: Polyhedron(vec![Face(vec![
                Point(Vector3::new(1., 0., 1.)),
                Point(Vector3::new(-1., 0., 1.)),
                Point(Vector3::new(-1., 0., -1.)),
                Point(Vector3::new(1., 0., -1.)),
            ])]),
            cut_axes: vec![CutAxis {
                names: CutAxisNames {
                    forward_name: "F",
                    middle_name: "S",
                    backward_name: "B",
                },
                expected_symmetry: None,
                normal: Unit::new_normalize(Vector3::new(1., 0., 0.)),
                distances: vec![0.5],
            }],
        };

        one_face.find_symmetries().unwrap();

        for cut_axis in one_face.cut_axes {
            assert_eq!(cut_axis.expected_symmetry, Some(2));
        }
    }

    #[test]
    fn symmetries() {
        let mut three_by_three = PuzzleDefinition {
            polyhedron: CUBE.to_owned(),
            cut_axes: vec![
                CutAxis {
                    names: CutAxisNames {
                        forward_name: "R",
                        middle_name: "M",
                        backward_name: "L",
                    },
                    expected_symmetry: None,
                    normal: Unit::new_normalize(Vector3::new(1., 0., 0.)),
                    distances: vec![1. / 3.],
                },
                CutAxis {
                    names: CutAxisNames {
                        forward_name: "U",
                        middle_name: "E",
                        backward_name: "D",
                    },
                    expected_symmetry: None,
                    normal: Unit::new_normalize(Vector3::new(0., 1., 0.)),
                    distances: vec![1. / 3.],
                },
                CutAxis {
                    names: CutAxisNames {
                        forward_name: "F",
                        middle_name: "S",
                        backward_name: "B",
                    },
                    expected_symmetry: None,
                    normal: Unit::new_normalize(Vector3::new(0., 0., 1.)),
                    distances: vec![1. / 3.],
                },
            ],
        };

        three_by_three.find_symmetries().unwrap();

        for cut_axis in three_by_three.cut_axes {
            assert_eq!(cut_axis.expected_symmetry, Some(4));
        }
    }
}
