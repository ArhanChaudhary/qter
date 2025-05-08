use std::{collections::VecDeque, iter, mem};

use internment::ArcIntern;
use itertools::Itertools;
use nalgebra::{Matrix2, Vector2, Vector3};

use crate::{E, Face, FaceSubspaceInfo, Point, PuzzleGeometryError};

/// Defines a generic cut surface; may or may not be planar or have only two regions.
///
/// Regions are represented by an `Option<ArcIntern<str>>`. A point "outside the region" can be represented by None. Having multiple regions in the same `CutSurface` is allowed.
pub trait CutSurface: core::fmt::Debug {
    /// Get the region that a point is in
    fn region(&self, point: Point) -> Option<ArcIntern<str>>;

    /// Tell whether a point is on the boundary of a cut plane
    fn on_boundary(&self, point: Point) -> bool;

    /// Get all points on boundaries intersected by the line segment spanned by the two points. The returned points must be in order from closest to A to closest to B, and `on_boundary` called on any of them must return `true`.
    fn boundaries_between(&self, point_a: Point, point_b: Point) -> Vec<Point>;

    /// Return a series of points that when connected as line segments including A and B, connects A and B through the boundary. A and B are guaranteed to already be on the boundary. `on_boundary` when called on any of the points must return `true`.
    fn join(&self, a: Point, b: Point, subspace_info: FaceSubspaceInfo) -> Vec<Point>;
}

#[derive(Clone, Debug)]
pub struct PlaneCut {
    pub spot: Vector3<f64>,
    pub normal: Vector3<f64>,
    pub name: ArcIntern<str>,
}

impl CutSurface for PlaneCut {
    fn region(&self, point: Point) -> Option<ArcIntern<str>> {
        let signum = self.normal.dot(&(point.0 - self.spot)).signum();
        assert!(signum == 1. || signum == -1.);
        if signum == 1. {
            Some(ArcIntern::clone(&self.name))
        } else {
            None
        }
    }

    fn on_boundary(&self, point: Point) -> bool {
        self.normal.dot(&(point.0 - self.spot)).abs() < E
    }

    fn boundaries_between(&self, a: Point, b: Point) -> Vec<Point> {
        let a_dot = self.normal.dot(&(a.0 - self.spot));
        let b_dot = self.normal.dot(&(b.0 - self.spot));

        if a_dot.signum() == b_dot.signum() {
            return vec![];
        }

        let frac = a_dot.abs() / (a_dot.abs() + b_dot.abs());

        let mut point = Point(a.0);
        point.0.axpy(frac, &b.0, 1.0 - frac);
        assert!(
            self.on_boundary(point),
            "{:?}, {}, {frac}",
            point,
            self.normal.dot(&(point.0 - self.spot))
        );

        vec![point]
    }

    fn join(&self, _: Point, _: Point, _: FaceSubspaceInfo) -> Vec<Point> {
        vec![]
    }
}

const I: Matrix2<f64> = Matrix2::new(0., -1., 1., 0.);

#[derive(Debug, Clone)]
struct Cycle<T>(VecDeque<T>);

impl<T> Cycle<T> {
    fn go_forward(&mut self) {
        let Some(value) = self.0.pop_front() else {
            return;
        };
        self.0.push_back(value);
    }

    fn go_backward(&mut self) {
        let Some(value) = self.0.pop_back() else {
            return;
        };
        self.0.push_front(value);
    }

    fn spot(&self) -> Option<&T> {
        self.0.front()
    }

    fn remove_spot(&mut self) -> Option<T> {
        self.0.pop_front()
    }

    fn prev(&self) -> Option<&T> {
        self.0.back()
    }

    fn prev_mut(&mut self) -> Option<&mut T> {
        self.0.back_mut()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn insert_before(&mut self, value: T) {
        self.0.push_back(value);
    }
}

pub(crate) fn do_cut<S: CutSurface + ?Sized>(
    surface: &S,
    face: &Face,
) -> Result<Vec<(Face, Option<ArcIntern<str>>)>, PuzzleGeometryError> {
    assert!(!face.points.is_empty());

    let subspace_info = face.subspace_info();

    // Convert the list of 3d points into a list of 2d edges, split on boundaries, with the edge's region included.
    let mut edges = Cycle(
        face.points
            .iter()
            .copied()
            .circular_tuple_windows()
            .take(face.points.len())
            .flat_map(|(a, b)| iter::once(a).chain(surface.boundaries_between(a, b)))
            .collect_vec()
            .iter()
            .circular_tuple_windows()
            .map(|(a, b)| {
                let middle = Point(a.0 / 2. + b.0 / 2.);

                (
                    (
                        subspace_info.make_2d * (a.0 - subspace_info.offset),
                        subspace_info.make_2d * (b.0 - subspace_info.offset),
                    ),
                    if surface.on_boundary(middle) {
                        None
                    } else {
                        Some(surface.region(middle))
                    },
                )
            })
            .collect::<VecDeque<_>>(),
    );

    let mut faces = Vec::new();

    while edges.len() >= 3 {
        // Merge collinear edges
        let mut i = 0;
        while i < edges.len() && edges.len() > 1 {
            let a = edges.prev().unwrap();
            let b = edges.spot().unwrap();
            if a.1 == b.1 && (I * (a.0.1 - a.0.0)).dot(&(b.0.1 - a.0.0)) < E {
                edges.prev_mut().unwrap().0.1 = b.0.1;
                edges.remove_spot();
                continue;
            }

            i += 1;
            edges.go_forward();
        }

        if edges.len() < 3 {
            break;
        }

        println!("{edges:#?}");

        faces.push(take_face_out(&mut edges, surface, face, subspace_info)?);
    }

    faces.retain(|v| v.0.is_valid().is_ok());

    Ok(faces)
}

fn take_face_out<S: CutSurface + ?Sized>(
    edges: &mut Cycle<((Vector2<f64>, Vector2<f64>), Option<Option<ArcIntern<str>>>)>,
    surface: &S,
    face: &Face,
    subspace_info: FaceSubspaceInfo,
) -> Result<(Face, Option<ArcIntern<str>>), PuzzleGeometryError> {
    // Find a collection of edges that can be merged
    // This algorithm tries to find a collection of vertices that "peeks out" and comes back to the same region.
    // If a collection of vertices didn't come back to the same region, then it would be impossible to merge them because we couldn't prove that they're the only vertices in the group.
    // As long as the regions don't have a cyclic structure, this should always happen somewhere.

    // Traverse to a region boundary
    let mut i = 0;

    while i < edges.len() {
        let prev_region = edges.prev().unwrap().1.as_ref();
        let region = edges.spot().unwrap().1.as_ref();

        if match region {
            Some(v) => Some(v) != prev_region && prev_region.is_some(),
            None => prev_region.is_some(),
        } {
            break;
        }

        edges.go_backward();
        i += 1;
    }

    if i == edges.len() {
        // All of the edges are in the same region

        let region = edges.spot().unwrap().1.clone().ok_or_else(|| {
            PuzzleGeometryError::CyclicalCutSurface(format!("{surface:?}"), face.to_owned())
        })?;

        return Ok((
            Face {
                points: mem::replace(edges, Cycle(VecDeque::new()))
                    .0
                    .into_iter()
                    .map(|v| Point(subspace_info.make_3d * v.0.0 + subspace_info.offset))
                    .collect_vec(),
                color: ArcIntern::clone(&face.color),
            },
            region,
        ));
    }

    // Scan the edges for regions that work

    #[allow(clippy::items_after_statements)]
    #[derive(Debug)]
    enum StateMachine {
        Begin {
            previous_region: Option<ArcIntern<str>>,
        },
        InRegion {
            previous_region: Option<ArcIntern<str>>,
            region: Option<ArcIntern<str>>,
        },
        SeeingWhatsNext {
            previous_region: Option<ArcIntern<str>>,
            region: Option<ArcIntern<str>>,
        },
    }

    i = 0;
    let mut state = StateMachine::Begin {
        previous_region: edges.prev().unwrap().1.as_ref().unwrap().to_owned(),
    };
    let mut found_region = None;
    // *2 because it may need to go over to check if a region is valid
    while i < edges.len() * 2 {
        state = match state {
            StateMachine::Begin { previous_region } => match &edges.spot().unwrap().1 {
                Some(region) => StateMachine::InRegion {
                    previous_region,
                    region: region.to_owned(),
                },
                None => StateMachine::Begin { previous_region },
            },
            StateMachine::InRegion {
                previous_region,
                region,
            } => match &edges.spot().unwrap().1 {
                Some(next_region) => {
                    if *next_region == region {
                        StateMachine::InRegion {
                            previous_region,
                            region,
                        }
                    } else if *next_region == previous_region {
                        found_region = Some(region);
                        break;
                    } else {
                        StateMachine::Begin {
                            previous_region: region,
                        }
                    }
                }
                None => StateMachine::SeeingWhatsNext {
                    previous_region,
                    region,
                },
            },
            StateMachine::SeeingWhatsNext {
                previous_region,
                region,
            } => match &edges.spot().unwrap().1 {
                Some(new_region) => {
                    if *new_region == previous_region || *new_region == region {
                        found_region = Some(region);
                        break;
                    }

                    StateMachine::Begin {
                        previous_region: region,
                    }
                }
                None => StateMachine::SeeingWhatsNext {
                    previous_region,
                    region,
                },
            },
        };
        i += 1;
        edges.go_forward();
    }

    // Split off the edges that are in the region

    let Some(found_region) = found_region else {
        return Err(PuzzleGeometryError::CyclicalCutSurface(
            format!("{surface:?}"),
            face.to_owned(),
        ));
    };

    edges.go_backward();

    println!("{:#?}", edges.spot().unwrap().1);
    while edges.spot().unwrap().1.is_none() {
        edges.go_backward();
        println!("{:#?}", edges.spot().unwrap().1);
    }

    let mut face_edges = Vec::new();

    while edges.spot().unwrap().1.as_ref() == Some(&found_region) {
        face_edges.push(edges.remove_spot().unwrap());
        edges.go_backward();
    }
    edges.go_forward();
    face_edges.reverse();

    // Turn the edges in the region into a face

    let mut points = face_edges
        .iter()
        .map(|v| v.0.0)
        .chain(iter::once(face_edges.last().unwrap().0.1))
        .map(|v| Point(subspace_info.make_3d * v + subspace_info.offset))
        .collect_vec();

    let first = *points.first().unwrap();
    let last = *points.last().unwrap();

    let mut joiner = VecDeque::from(surface.join(last, first, subspace_info));

    points.extend(joiner.iter());

    joiner.push_front(first);
    joiner.push_back(last);

    joiner
        .into_iter()
        .tuple_windows()
        .map(|(a, b)| {
            (
                (
                    subspace_info.make_2d * (a.0 - subspace_info.offset),
                    subspace_info.make_2d * (b.0 - subspace_info.offset),
                ),
                None,
            )
        })
        .for_each(|edge| {
            edges.insert_before(edge);
        });

    Ok((
        Face {
            points,
            color: ArcIntern::clone(&face.color),
        },
        found_region,
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use internment::ArcIntern;
    use nalgebra::Vector3;

    use crate::{Face, Point, PuzzleGeometryDefinition, do_cut, knife::PlaneCut, shapes::CUBE};

    #[test]
    fn plane_cut() {
        let face = Face {
            points: vec![
                Point(Vector3::new(1., 0., 1.)),
                Point(Vector3::new(1., 0., -1.)),
                Point(Vector3::new(-1., 0., -1.)),
                Point(Vector3::new(-1., 0., 1.)),
            ],
            color: ArcIntern::from("orange"),
        };

        let cutted = do_cut(
            &PlaneCut {
                spot: Vector3::new(0.5, 0., 0.),
                normal: Vector3::new(1., 0., 0.),
                name: ArcIntern::from("R"),
            },
            &face,
        )
        .unwrap();
        println!("{cutted:?}");

        assert_eq!(cutted.len(), 2);

        let face1 = Face {
            points: vec![
                Point(Vector3::new(1., 0., 1.)),
                Point(Vector3::new(1., 0., -1.)),
                Point(Vector3::new(0.5, 0., -1.)),
                Point(Vector3::new(0.5, 0., 1.)),
            ],
            color: ArcIntern::from("orange"),
        };

        let face2 = Face {
            points: vec![
                Point(Vector3::new(0.5, 0., 1.)),
                Point(Vector3::new(0.5, 0., -1.)),
                Point(Vector3::new(-1., 0., -1.)),
                Point(Vector3::new(-1., 0., 1.)),
            ],
            color: ArcIntern::from("orange"),
        };

        if cutted[0].0.epsilon_eq(&face1) {
            assert_eq!(cutted[0].1, Some(ArcIntern::from("R")));
            assert!(cutted[1].0.epsilon_eq(&face2));
            assert_eq!(cutted[1].1, None);
        } else {
            assert!(cutted[1].0.epsilon_eq(&face1));
            assert_eq!(cutted[1].1, Some(ArcIntern::from("R")));
            assert!(cutted[0].0.epsilon_eq(&face2));
            assert_eq!(cutted[0].1, None);
        }
    }

    #[test]
    fn three_by_three() {
        let cube = PuzzleGeometryDefinition {
            polyhedron: CUBE.to_owned(),
            cut_surfaces: vec![
                Arc::from(PlaneCut {
                    spot: Vector3::new(1. / 3., 0., 0.),
                    normal: Vector3::new(1., 0., 0.),
                    name: ArcIntern::from("L"),
                }),
                Arc::from(PlaneCut {
                    spot: Vector3::new(-1. / 3., 0., 0.),
                    normal: Vector3::new(-1., 0., 0.),
                    name: ArcIntern::from("R"),
                }),
                Arc::from(PlaneCut {
                    spot: Vector3::new(0., 1. / 3., 0.),
                    normal: Vector3::new(0., 1., 0.),
                    name: ArcIntern::from("U"),
                }),
                Arc::from(PlaneCut {
                    spot: Vector3::new(0., -1. / 3., 0.),
                    normal: Vector3::new(0., -1., 0.),
                    name: ArcIntern::from("D"),
                }),
                Arc::from(PlaneCut {
                    spot: Vector3::new(0., 0., 1. / 3.),
                    normal: Vector3::new(0., 0., 1.),
                    name: ArcIntern::from("F"),
                }),
                Arc::from(PlaneCut {
                    spot: Vector3::new(0., 0., -1. / 3.),
                    normal: Vector3::new(0., 0., -1.),
                    name: ArcIntern::from("B"),
                }),
            ],
        };

        let geometry = cube.geometry().unwrap();
        assert_eq!(geometry.stickers().len(), 54);
    }
}
