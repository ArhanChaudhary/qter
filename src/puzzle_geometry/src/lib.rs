mod puzzles;
use itertools::Itertools;
use nalgebra::{Matrix2, Matrix3x2, Vector3};
pub use puzzles::*;

mod puzzle_geometry;
pub use puzzle_geometry::*;
use thiserror::Error;

mod defaults;
mod options;

// Margin of error to consider points "equal"
const E: f64 = 0.000001;

type PuzzleDescriptionString<'a> = &'a str;

#[derive(Error, Debug)]
pub enum Error {
    #[error("The vertices of the face are not coplanar: {0:?}")]
    FaceNotCoplanar(Face),
    #[error("The face is not convex: {0:?}")]
    FaceNotConvex(Face),
    #[error("The face forms a line or a point rather than a plane: {0:?}")]
    FaceIsDegenerate(Face),
}

#[derive(Clone, Copy, Debug)]
pub struct Point(Vector3<f64>);

#[derive(Clone, Copy, Debug)]
pub struct Cut {
    // Direction is normal to the cut plane and the magnitude is the spacing between the cut planes
    pub normal: Vector3<f64>,
    // Cut like a 2x2 or 3x3? If false, the origin is cut, like a 2x2. If true, the origin is halfway between two cuts, like a 3x3.
    pub phase: bool,
}

#[derive(Clone, Debug)]
pub struct Face(pub Vec<Point>);

impl Face {
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

        // These two vectors define a 2d subspace that all points in the face should lie in
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
}

#[derive(Clone, Debug)]
pub struct Polyhedron(pub Vec<Face>);

#[derive(Clone, Debug)]
pub struct PuzzleDefinition {
    pub polyhedron: Polyhedron,
    pub cuts: Vec<Cut>,
}

pub fn puzzle_geometry(puzzle: PuzzleDefinition) -> Result<(), Error> {
    for face in &puzzle.polyhedron.0 {
        face.is_valid()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use nalgebra::Vector3;

    use crate::{Error, Face, Point};

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
}
