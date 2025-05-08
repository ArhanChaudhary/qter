#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::float_cmp)]

use std::sync::Arc;

use edge_cloud::EdgeCloud;
use internment::ArcIntern;
use itertools::Itertools;
use knife::{CutSurface, do_cut};
use ksolve::KSolve;
use nalgebra::{Matrix2x3, Matrix3, Matrix3x2, Rotation3, Unit, Vector3};
use qter_core::architectures::PermutationGroup;
use thiserror::Error;

mod edge_cloud;
pub mod knife;
pub mod ksolve;
pub mod shapes;

// Note... X is considered left-right, Y is considered up-down, and Z is considered front-back
//
// (minecraft style)

// Margin of error to consider points "equal"
const E: f64 = 1e-9;

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
    #[error(
        "The puzzle does not have {1}-fold rotational symmetry as expected by the cut line: {0:?}"
    )]
    PuzzleLacksExpectedSymmetry(Vector3<f64>, u8),
    #[error("The puzzle does not have any rotational symmetry along the cut line: {0:?}")]
    PuzzleLacksSymmetry(Vector3<f64>),
}

#[derive(Clone, Copy, Debug)]
pub struct Point(Vector3<f64>);

impl Point {
    fn rotated(self, axis: Vector3<f64>, symmetry: u8) -> Point {
        let rotation = rotation_of_degree(axis, symmetry);

        Point(rotation * self.0)
    }
}

fn rotation_of_degree(axis: Vector3<f64>, symm: u8) -> Matrix3<f64> {
    assert_ne!(symm, 0);

    Rotation3::from_axis_angle(
        &Unit::new_normalize(axis),
        core::f64::consts::TAU / f64::from(symm),
    )
    .into()
}

#[derive(Clone, Debug)]
pub struct Face {
    pub points: Vec<Point>,
    pub color: ArcIntern<str>,
}

impl Face {
    /// Rotate the face around the origin with the given axis and symmetry
    #[must_use]
    pub fn rotated(mut self, axis: Vector3<f64>, symmetry: u8) -> Face {
        let rotation = rotation_of_degree(axis, symmetry);

        for point in &mut self.points {
            point.0 = rotation * point.0;
        }

        self
    }

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
                let line = (b.0 - a.0).normalize();
                // Projection matrix onto the line spanned by the first two points
                let line_proj = line * line.transpose();

                (line_proj * (c.0 - a.0)).metric_distance(&(c.0 - a.0)) < E
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
        let plane_proj = make_3d * make_2d;

        for point in self.points.iter().skip(3) {
            let offsetted = point.0 - offset;
            if (plane_proj * offsetted).metric_distance(&offsetted) >= E {
                return Err(PuzzleGeometryError::FaceNotCoplanar(self.to_owned()));
            }
        }

        Ok(())
    }

    fn edge_cloud(&self) -> EdgeCloud {
        let mut cloud = Vec::new();

        for (vertex1, vertex2) in self
            .points
            .iter()
            .cycle()
            .tuple_windows()
            .take(self.points.len())
        {
            cloud.push((vertex1.0, vertex2.0));
        }

        EdgeCloud::new(vec![cloud])
    }

    fn epsilon_eq(&self, other: &Face) -> bool {
        self.edge_cloud().epsilon_eq(&other.edge_cloud())
    }

    /// Returns a pair of matrices where the first matrix projects a 2D vector into the 3D subspace spanned by this face, and the second computes the projection of a 3D vector into the 2D subspace.
    ///
    /// Also returns an origin vector to capture the translation of the face with respect to ⟨0, 0, 0⟩.
    fn subspace_info(&self) -> FaceSubspaceInfo {
        let offset = self.points[0].0;

        // These two vectors define a 3D subspace that all points in the face should lie in
        let basis1 = self.points[1].0 - offset;
        let basis2 = self.points[2].0 - offset;

        // Transforms a 2D space into the 3D subspace
        // Make it orthogonal because that's nice to have
        let make_3d = Matrix3x2::from_columns(&[basis1, basis2]).qr().q();
        // Project points in 3D space into the subspace and into the 2D space
        let make_2d = make_3d.pseudo_inverse(E).unwrap();

        FaceSubspaceInfo {
            make_3d,
            make_2d,
            offset,
        }
    }
}

/// Encodes the information about the plane on which a face lies.
#[derive(Clone, Copy, Debug)]
pub struct FaceSubspaceInfo {
    /// A matrix that converts a 2D vector to a 3D one in the subspace parallel to the face. To get a point on the face's plane, add `offset`.
    pub make_3d: Matrix3x2<f64>,
    /// Projects a 3D vector into the subspace parallel to the face. Given a point on the face's plane, subtract `offset` first.
    pub make_2d: Matrix2x3<f64>,
    /// The offset of the face from the origin. Subspaces must always include the origin due to how subspaces work mathematically so when projecting in/out, it is necessary to take the offset into account.
    pub offset: Vector3<f64>,
}

#[derive(Clone, Debug)]
pub struct Polyhedron(pub Vec<Face>);

#[derive(Clone, Debug)]
pub struct PuzzleGeometryDefinition {
    pub polyhedron: Polyhedron,
    pub cut_surfaces: Vec<Arc<dyn CutSurface>>,
}

#[derive(Clone, Debug)]
pub struct PuzzleGeometry {
    stickers: Vec<(Face, Vec<ArcIntern<str>>)>,
}

impl PuzzleGeometry {
    /// Get the puzzle as a permutation group over facelets
    #[must_use]
    pub fn permutation_group(&self) -> &PermutationGroup {
        todo!()
    }

    #[must_use]
    pub fn stickers(&self) -> &[(Face, Vec<ArcIntern<str>>)] {
        &self.stickers
    }

    /// Get the puzzle in its `KSolve` representation
    #[must_use]
    pub fn ksolve(&self) -> &KSolve {
        // Note: the KSolve permutation vector is **1-indexed**. See the test
        // cases for examples. It also exposes `zero_indexed_transformation` as
        // a convenience method.
        todo!()
    }
}

impl PuzzleGeometryDefinition {
    /// Consume a `PuzzleGeometryDefinition` and return a `PuzzleGeometry`
    ///
    /// # Errors
    ///
    /// If the validity of the faces is not satisfied, or if the puzzle does
    /// not have the expected symmetries, this function will return an error.
    pub fn geometry(self) -> Result<PuzzleGeometry, PuzzleGeometryError> {
        let mut faces: Vec<(Face, Vec<ArcIntern<str>>)> = vec![];
        for face in self.polyhedron.0 {
            face.is_valid()?;
            faces.push((face, vec![]));
        }

        for cut_surface in self.cut_surfaces {
            let mut new_faces = Vec::new();

            for (face, name_components) in faces {
                new_faces.extend(do_cut(&*cut_surface, &face)?.into_iter().map(
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

        Ok(PuzzleGeometry { stickers })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        Face, Point, PuzzleGeometryDefinition, PuzzleGeometryError, do_cut, knife::PlaneCut,
        shapes::CUBE,
    };
    use internment::ArcIntern;
    use nalgebra::Vector3;

    #[test]
    fn degeneracy() {
        let valid = Face {
            points: vec![Point(Vector3::new(1., 2., 3.))],
            color: ArcIntern::from("aliceblue"),
        }
        .is_valid();
        assert!(matches!(
            valid,
            Err(PuzzleGeometryError::FaceIsDegenerate(_))
        ));

        let valid = Face {
            points: vec![
                Point(Vector3::new(1., 2., 3.)),
                Point(Vector3::new(5., 4., 3.)),
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
                Point(Vector3::new(2., 2., 3.)),
                Point(Vector3::new(3., 4., 6.)),
                Point(Vector3::new(4., 6., 9.)),
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
                Point(Vector3::new(2., 2., 3.)),
                Point(Vector3::new(3., 4., 6.)),
                Point(Vector3::new(4., 6., 11.)),
                Point(Vector3::new(6., 6., 11.)),
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
                Point(Vector3::new(1., 1., 1.)),
                Point(Vector3::new(1., 1., 0.)),
                Point(Vector3::new(1., 0., 0.)),
                Point(Vector3::new(1., 0., 1.)),
            ],
            color: ArcIntern::from("bruh"),
        }
        .is_valid();

        assert!(matches!(valid, Ok(())));
    }

    /*
    #[test]
    fn symmetries_simple() {
        let mut one_face = PuzzleGeometryDefinition {
            polyhedron: Polyhedron(vec![Face {
                points: vec![
                    Point(Vector3::new(1., 0., 1.)),
                    Point(Vector3::new(-1., 0., 1.)),
                    Point(Vector3::new(-1., 0., -1.)),
                    Point(Vector3::new(1., 0., -1.)),
                ],
                color: ArcIntern::from("bruh"),
            }]),
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
    fn symmetries_3x3() {
        let mut three_by_three = PuzzleGeometryDefinition {
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

    #[test]
    fn symmetries_scuffed_3x3() {
        let mut three_by_three = PuzzleGeometryDefinition {
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
            assert_eq!(cut_axis.expected_symmetry, Some(2));
        }
    }

    #[test]
    fn symmetries_skewb() {
        let mut skewb = PuzzleGeometryDefinition {
            polyhedron: CUBE.to_owned(),
            cut_axes: vec![
                CutAxis {
                    names: CutAxisNames {
                        forward_name: "R",
                        middle_name: "M",
                        backward_name: "L",
                    },
                    expected_symmetry: None,
                    normal: Unit::new_normalize(Vector3::new(1., 1., 1.)),
                    distances: vec![0.],
                },
                CutAxis {
                    names: CutAxisNames {
                        forward_name: "U",
                        middle_name: "E",
                        backward_name: "D",
                    },
                    expected_symmetry: None,
                    normal: Unit::new_normalize(Vector3::new(-1., 1., 1.)),
                    distances: vec![0.],
                },
                CutAxis {
                    names: CutAxisNames {
                        forward_name: "F",
                        middle_name: "S",
                        backward_name: "B",
                    },
                    expected_symmetry: None,
                    normal: Unit::new_normalize(Vector3::new(1., 1., -1.)),
                    distances: vec![0.],
                },
                CutAxis {
                    names: CutAxisNames {
                        forward_name: "1",
                        middle_name: "2",
                        backward_name: "3",
                    },
                    expected_symmetry: None,
                    normal: Unit::new_normalize(Vector3::new(-1., 1., -1.)),
                    distances: vec![0.],
                },
            ],
        };

        skewb.find_symmetries().unwrap();

        for cut_axis in skewb.cut_axes {
            assert_eq!(cut_axis.expected_symmetry, Some(3));
        }
    }

    #[test]
    fn symmetries_pyraminx() {
        let up = Point(Vector3::new(0., 1., 0.));

        let down_1 = Point(
            Rotation3::from_axis_angle(
                &Unit::new_normalize(Vector3::new(1., 0., 0.)),
                (-1. / 3_f64).acos(),
            ) * up.0,
        );
        let down_2 = down_1.rotated(Vector3::new(0., 1., 0.), 3);
        let down_3 = down_2.rotated(Vector3::new(0., 1., 0.), 3);

        let mut pyraminx = PuzzleGeometryDefinition {
            polyhedron: TETRAHEDRON.to_owned(),
            cut_axes: vec![
                CutAxis {
                    names: CutAxisNames {
                        forward_name: "R",
                        middle_name: "M",
                        backward_name: "L",
                    },
                    expected_symmetry: None,
                    normal: Unit::new_normalize(up.0),
                    distances: vec![0., 0.5],
                },
                CutAxis {
                    names: CutAxisNames {
                        forward_name: "U",
                        middle_name: "E",
                        backward_name: "D",
                    },
                    expected_symmetry: None,
                    normal: Unit::new_normalize(down_1.0),
                    distances: vec![0., 0.5],
                },
                CutAxis {
                    names: CutAxisNames {
                        forward_name: "F",
                        middle_name: "S",
                        backward_name: "B",
                    },
                    expected_symmetry: None,
                    normal: Unit::new_normalize(down_2.0),
                    distances: vec![0., 0.5],
                },
                CutAxis {
                    names: CutAxisNames {
                        forward_name: "1",
                        middle_name: "2",
                        backward_name: "3",
                    },
                    expected_symmetry: None,
                    normal: Unit::new_normalize(down_3.0),
                    distances: vec![0., 0.5],
                },
            ],
        };

        pyraminx.find_symmetries().unwrap();

        for cut_axis in pyraminx.cut_axes {
            assert_eq!(cut_axis.expected_symmetry, Some(3));
        }
    }
    */
}
