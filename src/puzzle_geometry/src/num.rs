use std::{
    cmp::Ordering,
    iter::Sum,
    mem::{self, MaybeUninit},
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use algebraics::{algebraic_numbers::IntoRationalExponent, prelude::*};
use itertools::Itertools;

#[derive(Clone)]
pub struct Num(RealAlgebraicNumber);

impl Num {
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.cmp_with_zero() == Ordering::Equal
    }

    #[must_use]
    pub fn cmp_zero(&self) -> Ordering {
        self.0.cmp_with_zero()
    }

    #[must_use]
    pub fn pow<N: IntoRationalExponent>(self, pow: N) -> Num {
        Num(self.0.pow(pow))
    }

    #[must_use]
    pub fn sqrt(self) -> Num {
        self.pow((1, 2))
    }

    #[must_use]
    pub fn abs(self) -> Num {
        if self.cmp_zero().is_lt() { -self } else { self }
    }
}

impl core::fmt::Debug for Num {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            self.0.fmt(f)
        } else {
            let many_places = self.clone().0 * RealAlgebraicNumber::from(1_000_000_000_000_u64);
            let int = format!("{:0>13}", many_places.to_integer_trunc());
            let str = format!(
                "{}.{}{}",
                &int[..int.len() - 12],
                &int[int.len() - 12..],
                if many_places.is_integer() { "" } else { "..." }
            );
            let mut str = str.trim_end_matches('0');
            if str.ends_with('.') && !str.ends_with("...") {
                str = str.trim_end_matches('.');
            }
            f.write_str(str)
        }
    }
}

impl<T> From<T> for Num
where
    RealAlgebraicNumber: From<T>,
{
    fn from(value: T) -> Self {
        Self(RealAlgebraicNumber::from(value))
    }
}

impl AddAssign<Num> for Num {
    fn add_assign(&mut self, rhs: Num) {
        self.0 += rhs.0;
    }
}

impl Add<Num> for Num {
    type Output = Num;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl SubAssign<Num> for Num {
    fn sub_assign(&mut self, rhs: Num) {
        self.0 -= rhs.0;
    }
}

impl Sub<Num> for Num {
    type Output = Num;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self -= rhs;
        self
    }
}

impl MulAssign<Num> for Num {
    fn mul_assign(&mut self, rhs: Num) {
        self.0 *= rhs.0;
    }
}

impl Mul<Num> for Num {
    type Output = Num;

    fn mul(mut self, rhs: Self) -> Self::Output {
        self *= rhs;
        self
    }
}

impl DivAssign<Num> for Num {
    fn div_assign(&mut self, rhs: Num) {
        self.0 /= rhs.0;
    }
}

impl Div<Num> for Num {
    type Output = Num;

    fn div(mut self, rhs: Self) -> Self::Output {
        self /= rhs;
        self
    }
}

impl Neg for Num {
    type Output = Num;

    fn neg(self) -> Self::Output {
        Num(-self.0)
    }
}

impl Sum for Num {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Num(RealAlgebraicNumber::from(0_i64)), |a, v| a + v)
    }
}

impl PartialOrd for Num {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Num {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialEq for Num {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Num {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Matrix<const O: usize, const I: usize>([[Num; O]; I]);

pub type Vector<const N: usize> = Matrix<N, 1>;

impl<const N: usize> Vector<N> {
    #[must_use]
    pub fn norm(self) -> Num {
        let ns = self.norm_squared();
        ns.sqrt()
    }

    #[must_use]
    pub fn norm_squared(self) -> Num {
        let [v] = self.0;
        v.into_iter().map(|v| v.clone() * v).sum::<Num>()
    }

    #[must_use]
    pub fn normalize(mut self) -> Vector<N> {
        self.normalize_in_place();
        self
    }

    pub fn normalize_in_place(&mut self) {
        let norm = self.clone().norm();
        *self /= &norm;
    }

    #[must_use]
    pub fn dot(self, other: Vector<N>) -> Num {
        let [v1] = self.0;
        let [v2] = other.0;

        v1.into_iter()
            .zip(v2)
            .map(|(a, b)| a.clone() * b)
            .sum::<Num>()
    }

    #[must_use]
    pub fn proj_onto(self, other: Vector<N>) -> Vector<N> {
        let dot = self.dot(other.clone());
        let rescale = other.clone().norm_squared();
        other * &dot / &rescale
    }

    #[must_use]
    pub fn into_inner(self) -> [Num; N] {
        let [v] = self.0;
        v
    }

    #[must_use]
    pub fn inner(&self) -> &[Num; N] {
        let [v] = &self.0;
        v
    }
}

impl Vector<3> {
    #[must_use]
    #[expect(clippy::similar_names)]
    pub fn cross(self, other: Vector<3>) -> Vector<3> {
        let [v1x, v1y, v1z] = self.into_inner();
        let [v2x, v2y, v2z] = other.into_inner();

        Vector::new([[
            v1y.clone() * v2z.clone() - v1z.clone() * v2y.clone(),
            v1z * v2x.clone() - v1x.clone() * v2z,
            v1x * v2y - v1y * v2x,
        ]])
    }
}

impl<const O: usize, const I: usize> Matrix<O, I> {
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.iter().flatten().all(Num::is_zero)
    }

    #[must_use]
    pub fn zero() -> Self {
        Matrix::new([[0; O]; I])
    }

    /// Orthonormalize a matrix using the Gramm-Schmidt algorithm.
    ///
    /// # Panics
    /// The matrix must have full column rank
    #[must_use]
    pub fn mk_orthonormal(self) -> Matrix<O, I> {
        let mut columns = self.0.map(|col| Matrix([col]));

        for i in 0..I {
            for prev in 0..i {
                columns[i] -= columns[i].clone().proj_onto(columns[prev].clone());
            }

            assert!(
                !columns[i].is_zero(),
                "Matrix does not have full column rank: {columns:?}"
            );

            columns[i].normalize_in_place();
        }

        Matrix(columns.map(|col| {
            let [col] = col.0;
            col
        }))
    }

    #[must_use]
    pub fn transpose(self) -> Matrix<I, O> {
        let mut new_data = [const { [const { MaybeUninit::uninit() }; I] }; O];

        self.0.into_iter().enumerate().for_each(|(i, v)| {
            v.into_iter().enumerate().for_each(|(j, v)| {
                new_data[j][i] = MaybeUninit::new(v);
            });
        });

        Matrix(new_data.map(|v| v.map(|v| unsafe { v.assume_init() })))
    }
}

impl<const O: usize, const I: usize> Matrix<O, I> {
    pub fn new<N: Into<Num>>(data: [[N; O]; I]) -> Matrix<O, I> {
        Matrix(data.map(|v| v.map(Into::into)))
    }

    pub fn new_ratios<N: Into<RealAlgebraicNumber>>(data: [[(N, N); O]; I]) -> Matrix<O, I> {
        Matrix(data.map(|v| v.map(|(a, b)| Num(a.into() / b.into()))))
    }
}

impl<const O: usize, const I: usize> AddAssign<Matrix<O, I>> for Matrix<O, I> {
    fn add_assign(&mut self, rhs: Self) {
        self.0.iter_mut().zip(rhs.0).for_each(|(lhs, rhs)| {
            lhs.iter_mut().zip(rhs).for_each(|(lhs, rhs)| {
                *lhs = mem::replace(lhs, Num(RealAlgebraicNumber::zero())) + rhs;
            });
        });
    }
}

impl<const O: usize, const I: usize> Add<Matrix<O, I>> for Matrix<O, I> {
    type Output = Self;

    fn add(mut self, rhs: Matrix<O, I>) -> Self::Output {
        self += rhs;
        self
    }
}

impl<const O: usize, const I: usize> SubAssign<Matrix<O, I>> for Matrix<O, I> {
    fn sub_assign(&mut self, rhs: Self) {
        self.0.iter_mut().zip(rhs.0).for_each(|(lhs, rhs)| {
            lhs.iter_mut().zip(rhs).for_each(|(lhs, rhs)| {
                *lhs = mem::replace(lhs, Num(RealAlgebraicNumber::zero())) - rhs;
            });
        });
    }
}

impl<const O: usize, const I: usize> Sub<Matrix<O, I>> for Matrix<O, I> {
    type Output = Self;

    fn sub(mut self, rhs: Matrix<O, I>) -> Self::Output {
        self -= rhs;
        self
    }
}

impl<const O: usize, const I: usize> MulAssign<&Num> for Matrix<O, I> {
    fn mul_assign(&mut self, rhs: &Num) {
        self.0
            .iter_mut()
            .for_each(|v| v.iter_mut().for_each(|v| *v *= rhs.clone()));
    }
}

impl<const O: usize, const I: usize> Mul<&Num> for Matrix<O, I> {
    type Output = Self;

    fn mul(mut self, rhs: &Num) -> Self::Output {
        self *= rhs;
        self
    }
}

impl<const O: usize, const I: usize> DivAssign<&Num> for Matrix<O, I> {
    fn div_assign(&mut self, rhs: &Num) {
        self.0
            .iter_mut()
            .for_each(|v| v.iter_mut().for_each(|v| *v /= rhs.clone()));
    }
}

impl<const O: usize, const I: usize> Div<&Num> for Matrix<O, I> {
    type Output = Self;

    fn div(mut self, rhs: &Num) -> Self::Output {
        self /= rhs;
        self
    }
}

impl<const O: usize, const M: usize, const I: usize> Mul<&Matrix<M, I>> for &Matrix<O, M> {
    type Output = Matrix<O, I>;

    fn mul(self, rhs: &Matrix<M, I>) -> Self::Output {
        Matrix(
            (0..I)
                .map(|i| {
                    (0..O)
                        .map(|j| {
                            (0..M)
                                .map(|m| self.0[m][j].clone() * rhs.0[i][m].clone())
                                .sum::<Num>()
                        })
                        .collect_array()
                        .unwrap()
                })
                .collect_array()
                .unwrap(),
        )
    }
}

impl<const O: usize, const I: usize> Sum for Matrix<O, I> {
    fn sum<T: Iterator<Item = Self>>(iter: T) -> Self {
        iter.fold(Matrix::zero(), |a, v| a + v)
    }
}

#[must_use]
pub fn rotate_to(from: Matrix<3, 2>, to: Matrix<3, 2>) -> Matrix<3, 3> {
    // Let A be the matrix we want to return, F be `from`, and T be `to` (after orthonormalization and adding the third column)
    // We want...
    // AF = T
    // A = TF^-1
    // A = TF^T

    let from = from.mk_orthonormal();
    let to = to.mk_orthonormal();

    // Add a third column to prevent the final output from being underspecified
    let [v1, v2] = from.0.map(|v| Vector::new([v]));
    let v3 = v1.clone().cross(v2.clone());
    let from = Matrix::new([v1, v2, v3].map(Vector::into_inner));

    let [v1, v2] = to.0.map(|v| Vector::new([v]));
    let v3 = v1.clone().cross(v2.clone());
    let to = Matrix::new([v1, v2, v3].map(Vector::into_inner));

    &to * &from.transpose()
}

/// A rotation about an axis where the 2d subspace is rotated such that `(1, 0)` is rotated to `x_axis`
///
/// # Panics
///
/// `axis` and `x_axis` must not be zero
#[must_use]
pub fn rotation_about(axis: Vector<3>, x_axis: Vector<2>) -> Matrix<3, 3> {
    assert!(!x_axis.is_zero());
    assert!(!axis.is_zero());

    let [cos, sin] = x_axis.normalize().into_inner();
    let cosinv = Num::from(1) - cos.clone();

    let [x, y, z] = axis.normalize().into_inner();

    // https://en.wikipedia.org/wiki/Rotation_matrix#Rotation_matrix_from_axis_and_angle

    Matrix::new([
        [
            x.clone() * x.clone() * cosinv.clone() + cos.clone(),
            x.clone() * y.clone() * cosinv.clone() + z.clone() * sin.clone(),
            x.clone() * z.clone() * cosinv.clone() - y.clone() * sin.clone(),
        ],
        [
            y.clone() * x.clone() * cosinv.clone() - z.clone() * sin.clone(),
            y.clone() * y.clone() * cosinv.clone() + cos.clone(),
            y.clone() * z.clone() * cosinv.clone() + x.clone() * sin.clone(),
        ],
        [
            z.clone() * x.clone() * cosinv.clone() + y.clone() * sin.clone(),
            z.clone() * y * cosinv.clone() - x * sin,
            z.clone() * z * cosinv + cos,
        ],
    ])
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use crate::{
        DEG_72, DEG_90, DEG_120, DEG_180,
        num::{Num, Vector, rotate_to, rotation_about},
    };

    use super::Matrix;

    #[test]
    fn num_ops() {
        assert_eq!(Num::from(1) + Num::from(2), Num::from(3));
        assert_eq!(Num::from(1) - Num::from(2), Num::from(-1));
        assert_eq!(Num::from(4) * Num::from(3), Num::from(12));
        assert_eq!(Num::from(9) / Num::from(3), Num::from(3));

        assert!(Num::from(0).is_zero());
        assert_eq!(Num::from(0).cmp_zero(), Ordering::Equal);
        assert_eq!(Num::from(-1).cmp_zero(), Ordering::Less);
        assert_eq!(Num::from(1).cmp_zero(), Ordering::Greater);

        assert_eq!(Num::from(32).abs(), Num::from(32));
        assert_eq!(Num::from(-32).abs(), Num::from(32));

        assert_eq!(Num::from(64).sqrt(), Num::from(8));

        assert_eq!(
            [5, 4, 3, 2, 1].into_iter().map(Num::from).sum::<Num>(),
            Num::from(15)
        );
    }

    #[test]
    fn vector_ops() {
        assert_eq!(
            Vector::new([[1, 2, 3]]) * &Num::from(2),
            Vector::new([[2, 4, 6]])
        );
        assert_eq!(
            Vector::new([[3, 6, 9]]) / &Num::from(3),
            Vector::new([[1, 2, 3]]),
        );

        assert_eq!(
            Vector::new([[1, 2, 3]]) + Vector::new([[2, 4, 6]]),
            Vector::new([[3, 6, 9]])
        );

        assert!(!Vector::new([[0, 3, 0]]).is_zero());
        assert!(Vector::new([[0, 0, 0]]).is_zero());
        assert_eq!(Vector::zero(), Vector::new([[0, 0, 0]]));

        assert_eq!(Vector::new([[3, 4, 0]]).norm(), Num::from(5));
        assert_eq!(Vector::new([[3, 4, 0]]).norm_squared(), Num::from(25));

        assert_eq!(
            Vector::new([[3, 3, 0]]).normalize(),
            Vector::new([[1, 1, 0]]) / &Num::from(2).sqrt(),
        );

        assert_eq!(
            Vector::new([[1, 1, 0]]).dot(Vector::new([[0, 2, 1]])),
            Num::from(2)
        );

        assert_eq!(
            Vector::new([[1, 2, 0]]).proj_onto(Vector::new([[0, 100, 0]])),
            Vector::new([[0, 2, 0]])
        );

        assert_eq!(
            [[1, 2, 3], [2, 3, 4], [3, 4, 5]]
                .into_iter()
                .map(|v| Vector::new([v]))
                .sum::<Vector<3>>(),
            Vector::new([[6, 9, 12]])
        );

        for v in [&*DEG_180, &*DEG_120, &*DEG_90, &*DEG_72] {
            assert_eq!(v, &v.clone().normalize());
        }
    }

    #[test]
    fn matrix_ops() {
        assert_eq!(
            Matrix::new([[3, 0, 0], [5, 2, 0], [42, 10, 91]]).mk_orthonormal(),
            Matrix::new([[1, 0, 0], [0, 1, 0], [0, 0, 1]])
        );

        assert_eq!(
            &Matrix::new([[1, 0, 0], [0, 0, 1], [0, 1, 0]]) * &Vector::new([[1, 2, 3]]),
            Vector::new([[1, 3, 2]])
        );

        assert_eq!(
            &Matrix::new([[5, 2, 9], [3, 9, 0], [2, 4, 3]])
                * &Matrix::new([[9, 3, 4], [2, 5, 1], [6, 2, 1]]),
            Matrix::new([[62, 61, 93], [27, 53, 21], [38, 34, 57]])
        );

        assert_eq!(
            Matrix::new([[1, 2, 3], [4, 5, 6]]).transpose(),
            Matrix::new([[1, 4], [2, 5], [3, 6]])
        );
    }

    #[test]
    fn test_rotate_to() {
        assert_eq!(
            rotate_to(
                Matrix::new([[1, 0, 0], [0, 1, 0]]),
                Matrix::new([[0, 1, 0], [0, 0, 1]])
            ),
            Matrix::new([[0, 1, 0], [0, 0, 1], [1, 0, 0]])
        );

        assert_eq!(
            rotate_to(
                Matrix::new([[0, 2, 0], [0, 0, 4]]),
                Matrix::new([[0, 0, 3], [1, 0, 2]])
            ),
            Matrix::new([[0, 1, 0], [0, 0, 1], [1, 0, 0]])
        );
    }

    #[test]
    fn test_rotation_about() {
        assert_eq!(
            rotation_about(Vector::new([[0, 1, 0]]), Vector::new([[0, 1]])),
            Matrix::new([[0, 0, -1], [0, 1, 0], [1, 0, 0]])
        );

        assert_eq!(
            rotation_about(Vector::new([[0, 0, 1]]), Vector::new([[-1, 0]])),
            Matrix::new([[-1, 0, 0], [0, -1, 0], [0, 0, 1]])
        );
    }
}
