use cycle_combination_solver::phase2;
use edge_cloud::EdgeCloud;
use itertools::Itertools;
use nalgebra::{Matrix3, Matrix3x2, Rotation3, Unit, Vector3};
use qter_core::architectures::PermutationGroup;
use thiserror::Error;

mod edge_cloud;
pub mod puzzles;

// Note... X is considered left-right, Y is considered up-down, and Z is considered front-back
//
// (minecraft style)

// Margin of error to consider points "equal"
const E: f64 = 1e-9;

type PuzzleDescriptionString<'a> = &'a str;

#[derive(Error, Debug)]
pub enum Error {
    #[error("The vertices of the face are not coplanar: {0:?}")]
    FaceNotCoplanar(Face),
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

impl Point {
    fn rotated(self, axis: Vector3<f64>, symmetry: u8) -> Point {
        let rotation = rotation_of_degree(axis, symmetry);

        Point(rotation * self.0)
    }
}

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
    assert_ne!(symm, 0);

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

        Ok(())
    }

    fn edge_cloud(&self) -> EdgeCloud {
        let mut cloud = Vec::new();

        for (vertex1, vertex2) in self.0.iter().cycle().tuple_windows().take(self.0.len()) {
            cloud.push((vertex1.0, vertex2.0));
        }

        EdgeCloud::new(vec![cloud])
    }

    fn epsilon_eq(&self, other: &Face) -> bool {
        self.edge_cloud().epsilon_eq(&other.edge_cloud())
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
    pub fn piece_perm_and_ori_group<S: phase2::puzzle::Storage>(&self) -> &PiecePermAndOriGroup<S> {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct PiecePermAndOriGroup<S: phase2::puzzle::Storage> {
    _marker: std::marker::PhantomData<S>,
}

pub trait PuzzleGeometryInterface<S: phase2::puzzle::Storage> {
    fn pieces(&self) -> &[(usize, u8)];
    fn moves(&self) -> &[phase2::puzzle::Move<S>];
    fn symmetries(&self) -> &[phase2::puzzle::PuzzleState<S>];
}

impl<S: phase2::puzzle::Storage> PuzzleGeometryInterface<S> for PiecePermAndOriGroup<S> {
    /// For each type of piece, return a list of (amount of the piece type, orientation mod)
    fn pieces(&self) -> &[(usize, u8)] {
        todo!()
    }

    /// Get the set of available moves on the puzzle
    fn moves(&self) -> &[phase2::puzzle::Move<S>] {
        todo!()
    }

    /// Get the list of symmetries obeyed by the puzzle
    fn symmetries(&self) -> &[phase2::puzzle::PuzzleState<S>] {
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

                    if !cloud.try_symmetry(&mut into, matrix) {
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

                        if cloud.try_symmetry(&mut into, matrix) {
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
    fn edge_cloud(&self) -> EdgeCloud {
        let mut edges = Vec::new();
        let mut cuts = Vec::new();
        let mut zero_axes = Vec::new();

        // TODO: Handle these cuts separately, mixing them in with the edges won't work in general

        for cut_axis in &self.cut_axes {
            // Cuts must also have the correct symmetry, but they are automatically rotationally symmetric with themselves
            for cut in &cut_axis.distances {
                if *cut < E {
                    // In the case of just a cut through the middle, this will still ensure symmetry.
                    // We can't push this directly because the direction will be inaccessible if the magnitude is zero and zero cuts in different directions will count
                    zero_axes.push((*cut_axis.normal, -*cut_axis.normal));
                    continue;
                }

                let cut_spot = *cut_axis.normal * *cut;
                cuts.push((cut_spot, -cut_spot));
            }
        }

        for face in &self.polyhedron.0 {
            edges.extend_from_slice(&face.edge_cloud().sections()[0]);
        }

        EdgeCloud::new(vec![zero_axes, cuts, edges])
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::{Rotation3, Unit, Vector3};

    use crate::{
        puzzles::{CUBE, TETRAHEDRON},
        CutAxis, CutAxisNames, Error, Face, Point, Polyhedron, PuzzleDefinition,
    };

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
    fn symmetries_3x3() {
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

    #[test]
    fn symmetries_scuffed_3x3() {
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
        let mut skewb = PuzzleDefinition {
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

        let mut pyraminx = PuzzleDefinition {
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
}
