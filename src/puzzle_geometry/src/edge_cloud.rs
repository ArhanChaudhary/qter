use std::{cmp::Ordering, mem};

use itertools::Itertools;

use crate::num::{Matrix, Vector};

#[derive(Clone, Debug)]
pub struct EdgeCloud {
    edges: Vec<(Vector<3>, Vector<3>)>,
}

impl EdgeCloud {
    pub fn new(mut edges: Vec<(Vector<3>, Vector<3>)>) -> EdgeCloud {
        sort_edge_cloud(&mut edges);

        EdgeCloud { edges }
    }

    pub fn try_symmetry(&self, into: &mut EdgeCloud, matrix: &Matrix<3, 3>) -> bool {
        let cloud: &[(Vector<3>, Vector<3>)] = &self.edges;
        let into: &mut [(Vector<3>, Vector<3>)] = &mut into.edges;
        assert!(
            into.len() == cloud.len(),
            "The temporary buffer must have identical dimensions to the real data"
        );

        into.clone_from_slice(cloud);

        for point in into.iter_mut().flat_map(|(a, b)| [a, b]) {
            *point = matrix * point;
        }

        sort_edge_cloud(into);

        edge_cloud_eq(cloud, into)
    }

    pub fn epsilon_eq(&self, other: &EdgeCloud) -> bool {
        edge_cloud_eq(&self.edges, &other.edges)
    }

    pub fn edges(&self) -> &[(Vector<3>, Vector<3>)] {
        &self.edges
    }
}

fn sort_edge_cloud(cloud: &mut [(Vector<3>, Vector<3>)]) {
    for (a, b) in &mut *cloud {
        let ordering = a
            .inner()
            .iter()
            .zip(b.inner().iter())
            .map(|(x1, x2)| x1.cmp(x2))
            .find_or_last(|v| !matches!(v, Ordering::Equal))
            .unwrap();

        if matches!(ordering, Ordering::Greater) {
            mem::swap(a, b);
        }
    }

    cloud.sort_unstable_by(|(a1, b1), (a2, b2)| {
        a1.inner()
            .iter()
            .zip(a2.inner().iter())
            .chain(b1.inner().iter().zip(b2.inner().iter()))
            .map(|(x1, x2)| x1.cmp(x2))
            .find_or_last(|v| !matches!(v, Ordering::Equal))
            .unwrap()
    });
}

fn edge_cloud_eq(cloud1: &[(Vector<3>, Vector<3>)], cloud2: &[(Vector<3>, Vector<3>)]) -> bool {
    cloud1
        .iter()
        .zip(cloud2)
        .all(|((a1, b1), (a2, b2))| a1 == a2 && b1 == b2)
}

#[cfg(test)]
mod tests {
    use crate::{
        DEG_72, DEG_120,
        num::{Vector, rotation_about},
        shapes::TETRAHEDRON,
    };

    use super::EdgeCloud;

    #[test]
    fn equality() {
        let edge_cloud_one = EdgeCloud::new(vec![
            (Vector::new([[1, 2, 3]]), Vector::new([[4, 5, 6]])),
            (Vector::new([[3, 2, 1]]), Vector::new([[6, 5, 4]])),
            (Vector::new([[4, 8, 3]]), Vector::new([[2, 5, 6]])),
        ]);

        let edge_cloud_two = EdgeCloud::new(vec![
            (Vector::new([[4, 5, 6]]), Vector::new([[1, 2, 3]])),
            (Vector::new([[4, 8, 3]]), Vector::new([[2, 5, 6]])),
            (Vector::new([[6, 5, 4]]), Vector::new([[3, 2, 1]])),
        ]);

        assert!(edge_cloud_one.epsilon_eq(&edge_cloud_two));
    }

    #[test]
    fn try_symmetry() {
        let tetrahedron = EdgeCloud::new(
            TETRAHEDRON
                .0
                .iter()
                .flat_map(|v| v.edge_cloud().edges().to_vec())
                .collect(),
        );

        let mut buffer = tetrahedron.clone();

        assert!(tetrahedron.try_symmetry(
            &mut buffer,
            &rotation_about(Vector::new([[0, 1, 0]]), DEG_120.clone()),
        ));

        assert!(!tetrahedron.try_symmetry(
            &mut buffer,
            &rotation_about(Vector::new([[0, 1, 0]]), DEG_72.clone()),
        ));
    }
}
