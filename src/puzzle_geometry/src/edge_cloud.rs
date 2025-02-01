use std::{cmp::Ordering, mem};

use itertools::Itertools;
use nalgebra::{Matrix3, Vector3};

use crate::E;

#[derive(Clone, Debug)]
pub struct EdgeCloud {
    sections: Vec<Vec<(Vector3<f64>, Vector3<f64>)>>,
}

impl EdgeCloud {
    pub fn new(mut sections: Vec<Vec<(Vector3<f64>, Vector3<f64>)>>) -> EdgeCloud {
        for section in &mut sections {
            sort_edge_cloud(section);
        }

        EdgeCloud { sections }
    }

    pub fn try_symmetry(&self, into: &mut EdgeCloud, matrix: Matrix3<f64>) -> bool {
        self.sections
            .iter()
            .zip(into.sections.iter_mut())
            .all(|(section, into)| try_symmetry(section, into, matrix))
    }

    pub fn epsilon_eq(&self, other: &EdgeCloud) -> bool {
        if self.sections.len() != other.sections.len() {
            return false;
        }

        self.sections
            .iter()
            .zip(other.sections.iter())
            .all(|(section, other)| edge_cloud_eq(section, other))
    }

    pub fn sections(&self) -> &[Vec<(Vector3<f64>, Vector3<f64>)>] {
        &self.sections
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
