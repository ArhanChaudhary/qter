//! The point of this module is to define a generic number type so that we can try out different number types without refactoring. I'm most interested in arbitrary size integers so that we can represent arbitrarily large orders (megaminx) but that would come with a performance penalty since we lose the Copy implementation.
use std::{
    cmp::Ordering,
    fmt::{Debug, Display},
    iter::{Product, Sum},
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign},
    str::FromStr,
};

use bnum::types::{I512, U512};

/// Signed
pub struct I;
/// Unsigned
pub struct U;

/// A signed or unsigned integer
pub struct Int<Signed> {
    value: I512,
    phantom: PhantomData<Signed>,
}

impl<Signed> Int<Signed> {
    /// Returns `true` if the value is zero and `false` otherwise
    pub fn is_zero(&self) -> bool {
        self.value == I512::ZERO
    }

    pub fn zero() -> Int<Signed> {
        Int {
            value: I512::ZERO,
            phantom: PhantomData,
        }
    }

    pub fn one() -> Int<Signed> {
        Int {
            value: I512::ONE,
            phantom: PhantomData,
        }
    }

    fn from_inner(value: I512) -> Int<Signed> {
        Int {
            value,
            phantom: PhantomData,
        }
    }

    #[cfg(test)]
    pub fn to_u64(&self) -> u64 {
        use bnum::cast::As;

        self.value.as_()
    }

    #[cfg(test)]
    pub fn to_i64(&self) -> i64 {
        use bnum::cast::As;

        self.value.as_()
    }
}

impl<Signed> Clone for Int<Signed> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Signed> Copy for Int<Signed> {}

impl<Signed> Debug for Int<Signed> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", core::any::type_name::<Signed>(), self)
    }
}

impl<Signed> Display for Int<Signed> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.value, f)
    }
}

pub struct NumberOutOfRange<Signed> {
    value: Int<Signed>,
    number_ty: &'static str,
    min: Int<I>,
    max: Int<I>,
}

impl<Signed> Debug for NumberOutOfRange<Signed> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl<Signed> Display for NumberOutOfRange<Signed> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The number {} is out of range for values of type {} that must be between {} and {}.",
            self.value, self.number_ty, self.min, self.max
        )
    }
}

impl FromStr for Int<I> {
    type Err = bnum::errors::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_inner(s.trim().parse()?))
    }
}

impl FromStr for Int<U> {
    type Err = bnum::errors::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num: U512 = s.trim().parse()?;
        let num: I512 = num.to_string().parse()?;

        Ok(Self::from_inner(num))
    }
}

impl From<Int<U>> for Int<I> {
    fn from(value: Int<U>) -> Self {
        Int::from_inner(value.value)
    }
}

macro_rules! from {
    (unsigned $ty: ty) => {
        impl<Signed> From<$ty> for Int<Signed> {
            fn from(value: $ty) -> Self {
                Int::from_inner(I512::from(value))
            }
        }
    };

    (signed $ty: ty) => {
        impl From<$ty> for Int<I> {
            fn from(value: $ty) -> Self {
                Int::from_inner(I512::from(value))
            }
        }
    };
}

from!(unsigned u64);
from!(unsigned u32);
from!(unsigned u16);
from!(unsigned u8);
from!(unsigned usize);
from!(signed i64);
from!(signed i32);
from!(signed i16);
from!(signed i8);
from!(signed isize);

macro_rules! try_from {
    ($ty: ty) => {
        impl<Signed> TryFrom<Int<Signed>> for $ty {
            type Error = NumberOutOfRange<Signed>;

            fn try_from(value: Int<Signed>) -> Result<Self, Self::Error> {
                if value < Int::<I>::from(<$ty>::MIN) || value > Int::<I>::from(<$ty>::MAX) {
                    return Err(NumberOutOfRange {
                        value,
                        number_ty: core::any::type_name::<$ty>(),
                        min: Int::from(<$ty>::MIN),
                        max: Int::from(<$ty>::MAX),
                    });
                }

                Ok(bnum::cast::As::as_(value.value))
            }
        }
    };
}

try_from!(u64);
try_from!(u32);
try_from!(u16);
try_from!(u8);
try_from!(usize);
try_from!(i64);
try_from!(i32);
try_from!(i16);
try_from!(i8);
try_from!(isize);

macro_rules! impl_signed_variants {
    ($op: ident, $name: ident, $op_assign: ident, $name_assign: ident, |$a: ident, $b: ident| signed $signed_code: expr, unsigned $unsigned_code: expr) => {
        impl<Signed> $op<Int<I>> for Int<Signed> {
            type Output = Int<I>;

            fn $name(self, rhs: Int<I>) -> Int<I> {
                let $a = self.value;
                let $b = rhs.value;
                Int::from_inner($signed_code)
            }
        }

        impl $op<Int<U>> for Int<I> {
            type Output = Int<I>;

            fn $name(self, rhs: Int<U>) -> Int<I> {
                let $a = self.value;
                let $b = rhs.value;
                Int::from_inner($signed_code)
            }
        }

        impl $op<Int<U>> for Int<U> {
            type Output = Int<U>;

            fn $name(self, rhs: Int<U>) -> Int<U> {
                let $a = self.value;
                let $b = rhs.value;
                Int::from_inner($unsigned_code)
            }
        }

        impl<Signed> $op_assign<Int<I>> for Int<Signed> {
            fn $name_assign(&mut self, rhs: Int<I>) {
                let $a = self.value;
                let $b = rhs.value;
                self.value = $signed_code;
            }
        }

        impl $op_assign<Int<U>> for Int<I> {
            fn $name_assign(&mut self, rhs: Int<U>) {
                let $a = self.value;
                let $b = rhs.value;
                self.value = $signed_code;
            }
        }

        impl $op_assign<Int<U>> for Int<U> {
            fn $name_assign(&mut self, rhs: Int<U>) {
                let $a = self.value;
                let $b = rhs.value;
                self.value = $unsigned_code;
            }
        }
    };
}

impl_signed_variants!(Add, add, AddAssign, add_assign, |a, b| signed a + b, unsigned a + b);
impl_signed_variants!(Mul, mul, MulAssign, mul_assign, |a, b| signed a * b, unsigned a * b);
impl_signed_variants!(Sub, sub, SubAssign, sub_assign, |a, b| signed a - b, unsigned {
    let v = a - b;

    if v < I512::ZERO {
        panic!("Attempted to subtract with underflow!")
    }

    v
});

// Euclidean division and remainder are more reasonable defaults for what we're doing

impl_signed_variants!(Div, div, DivAssign, div_assign, |a, b| signed a.div_euclid(b), unsigned a / b);

// Euclidean remainder always gives a nonnegative value, so always return unsigned

impl<Signed> Rem<Int<I>> for Int<Signed> {
    type Output = Int<U>;
    fn rem(self, rhs: Int<I>) -> Int<U> {
        Int::from_inner(self.value.rem_euclid(rhs.value))
    }
}
impl Rem<Int<U>> for Int<I> {
    type Output = Int<U>;
    fn rem(self, rhs: Int<U>) -> Int<U> {
        Int::from_inner(self.value.rem_euclid(rhs.value))
    }
}
impl Rem<Int<U>> for Int<U> {
    type Output = Int<U>;
    fn rem(self, rhs: Int<U>) -> Int<U> {
        Int::from_inner(self.value % rhs.value)
    }
}
impl<Signed> RemAssign<Int<I>> for Int<Signed> {
    fn rem_assign(&mut self, rhs: Int<I>) {
        self.value = self.value.rem_euclid(rhs.value);
    }
}
impl RemAssign<Int<U>> for Int<I> {
    fn rem_assign(&mut self, rhs: Int<U>) {
        self.value = self.value.rem_euclid(rhs.value);
    }
}
impl RemAssign<Int<U>> for Int<U> {
    fn rem_assign(&mut self, rhs: Int<U>) {
        self.value = self.value % rhs.value;
    }
}

impl<SignedA, SignedB> PartialEq<Int<SignedA>> for Int<SignedB> {
    fn eq(&self, other: &Int<SignedA>) -> bool {
        self.value == other.value
    }
}

impl<Signed> Eq for Int<Signed> {}

impl<SignedA, SignedB> PartialOrd<Int<SignedA>> for Int<SignedB> {
    fn partial_cmp(&self, other: &Int<SignedA>) -> Option<Ordering> {
        Some(self.value.cmp(&other.value))
    }
}

impl<Signed> Ord for Int<Signed> {
    fn cmp(&self, other: &Int<Signed>) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<Signed> Neg for Int<Signed> {
    type Output = Int<I>;

    fn neg(self) -> Int<I> {
        Int {
            value: -self.value,
            phantom: PhantomData,
        }
    }
}

impl Sum for Int<U> {
    fn sum<V: Iterator<Item = Self>>(iter: V) -> Self {
        let mut accumulator = Int::<U>::zero();

        for item in iter {
            accumulator += item;
        }

        accumulator
    }
}

impl Sum for Int<I> {
    fn sum<V: Iterator<Item = Self>>(iter: V) -> Self {
        let mut accumulator = Int::<I>::zero();

        for item in iter {
            accumulator += item;
        }

        accumulator
    }
}

impl Product for Int<U> {
    fn product<V: Iterator<Item = Self>>(iter: V) -> Self {
        let mut accumulator = Int::<U>::zero();

        for item in iter {
            accumulator *= item;
        }

        accumulator
    }
}

impl Product for Int<I> {
    fn product<V: Iterator<Item = Self>>(iter: V) -> Self {
        let mut accumulator = Int::<I>::zero();

        for item in iter {
            accumulator *= item;
        }

        accumulator
    }
}
