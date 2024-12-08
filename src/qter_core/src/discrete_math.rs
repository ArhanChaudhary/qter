use bnum::{
    cast::As,
    types::{I512, U512},
};

/// Calculate the GCD of two numbers
pub fn gcd(mut a: U512, mut b: U512) -> U512 {
    loop {
        if b == U512::ZERO {
            return a;
        }

        let rem = a.rem_euclid(b);
        a = b;
        b = rem;
    }
}

/// Calculate the LCM of two numbers
pub fn lcm(a: U512, b: U512) -> U512 {
    assert_ne!(a, U512::ZERO);
    assert_ne!(b, U512::ZERO);

    b / gcd(a, b) * a
}

/// Calculate the LCM of a list of numbers
pub fn lcm_iter(values: impl Iterator<Item = U512>) -> U512 {
    values.fold(U512::ONE, lcm)
}

/// Calculate the GCD of two numbers as well as the coefficients of Bézout's identity
pub fn extended_euclid(mut a: U512, mut b: U512) -> ((I512, I512), U512) {
    let mut a_coeffs = (I512::ONE, I512::ZERO);
    let mut b_coeffs = (I512::ZERO, I512::ONE);

    loop {
        if b == U512::ZERO {
            return (a_coeffs, a);
        }

        let to_sub = a.div_euclid(b);
        let to_sub_s = to_sub.as_::<I512>();
        let new_coeffs = (
            a_coeffs.0 - b_coeffs.0 * to_sub_s,
            a_coeffs.1 - b_coeffs.1 * to_sub_s,
        );
        let rem = a - b * to_sub;
        a = b;
        a_coeffs = b_coeffs;
        b = rem;
        b_coeffs = new_coeffs;
    }
}

// Implementation based on https://math.stackexchange.com/questions/1644677/what-to-do-if-the-modulus-is-not-coprime-in-the-chinese-remainder-theorem
/// Calculate the chinese remainder theorem over a list of tuples of remainders with moduli. The return value is bounded by the LCM of the moduli.
///
/// For each `(k, m) ∈ conditions`, the return value is congruent to `k mod m`.
///
/// This is a generalization of the CRT that supports moduli that aren't coprime. Because of this, a value that satifies all of the conditions is not guaranteed. If the conditions contradict each other, the function will return `None`.
///
/// If any of the conditions give `None`, the function will stop and return `None`.
pub fn chinese_remainder_theorem(
    mut conditions: impl Iterator<Item = Option<(U512, U512)>>,
) -> Option<U512> {
    let (mut prev_remainder, mut prev_modulus) = match conditions.next() {
        Some(Some(condition)) => condition,
        Some(None) => return None,
        None => return Some(U512::ZERO),
    };

    for cond in conditions {
        let (remainder, modulus) = cond?;

        let (coeffs, gcd) = extended_euclid(prev_modulus, modulus);

        let diff = if remainder > prev_remainder {
            remainder - prev_remainder
        } else {
            prev_remainder - remainder
        };

        if diff.rem_euclid(gcd) != U512::ZERO {
            return None;
        }

        let λ = diff / gcd;

        let x = if remainder > prev_remainder {
            remainder.as_::<I512>() - modulus.as_::<I512>() * coeffs.1 * λ.as_::<I512>()
        } else {
            prev_remainder.as_::<I512>() - prev_modulus.as_::<I512>() * coeffs.0 * λ.as_::<I512>()
        };

        let new_modulus = lcm(prev_modulus, modulus);

        prev_remainder = x.rem_euclid(new_modulus.as_()).as_();
        prev_modulus = new_modulus;
    }

    Some(prev_remainder)
}

/// This function does what it says on the tin.
///
/// "AAAA"  → 1
/// "ABAB"  → 2
/// "ABCA"  → 4
/// "ABABA" → 5
///
/// Every string given by the iterator is treated as a unit rather than split apart, so `["Yellow", "Green", "Yellow", "Green"]` would return `2`.
///
/// This function is important for computing the chromatic order of cycles.
pub fn length_of_substring_that_this_string_is_n_repeated_copies_of<'a>(
    colors: impl Iterator<Item = &'a str>,
) -> U512 {
    let mut found = vec![];
    let mut current_repeat_length = 1;

    for (i, color) in colors.enumerate() {
        found.push(color);

        if found[i % current_repeat_length] != color {
            current_repeat_length = i + 1;
        }
    }

    // We didn't match the substring a whole number of times; it actually doesn't work
    if found.len() % current_repeat_length != 0 {
        current_repeat_length = found.len();
    }

    U512::from_digit(current_repeat_length as u64)
}

#[cfg(test)]
mod tests {
    use bnum::{cast::As, types::U512};

    use crate::discrete_math::{
        extended_euclid, gcd, lcm, length_of_substring_that_this_string_is_n_repeated_copies_of,
    };

    use super::chinese_remainder_theorem;

    #[test]
    fn lcm_and_gcd() {
        let _lcm = |a, b| lcm(U512::from_digit(a), U512::from_digit(b)).digits()[0];
        let _gcd = |a, b| gcd(U512::from_digit(a), U512::from_digit(b)).digits()[0];
        let _ext_euc = |a, b| {
            let ((x, y), z) = extended_euclid(U512::from_digit(a), U512::from_digit(b));
            assert_eq!(
                a as i64 * x.as_::<i64>() + b as i64 * y.as_::<i64>(),
                z.as_::<i64>()
            );
            z.as_::<u64>()
        };

        assert_eq!(_gcd(3, 5), 1);
        assert_eq!(_gcd(3, 6), 3);
        assert_eq!(_gcd(4, 6), 2);

        assert_eq!(_ext_euc(3, 5), 1);
        assert_eq!(_ext_euc(3, 6), 3);
        assert_eq!(_ext_euc(4, 6), 2);

        assert_eq!(_lcm(3, 5), 15);
        assert_eq!(_lcm(3, 6), 6);
        assert_eq!(_lcm(4, 6), 12);
    }

    fn _crt(v: impl IntoIterator<Item = (u64, u64)>) -> Option<u64> {
        chinese_remainder_theorem(
            v.into_iter()
                .map(|(a, b)| Some((U512::from_digit(a), U512::from_digit(b)))),
        )
        .map(|v| v.digits()[0])
    }

    #[test]
    fn crt() {
        assert_eq!(_crt([(2, 3), (1, 2)]), Some(5));
        assert_eq!(_crt([(3, 4), (1, 2)]), Some(3));
        assert_eq!(_crt([(3, 4), (1, 2), (3, 5), (4, 7)]), Some(123));
        assert_eq!(_crt([(2, 4), (1, 2)]), None);
    }

    #[test]
    fn length_of_substring_whatever() {
        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "a", "a", "a"].into_iter()
            )
            .digits()[0],
            1
        );

        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "b", "a", "b"].into_iter()
            )
            .digits()[0],
            2
        );

        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "b", "a", "b", "a"].into_iter()
            )
            .digits()[0],
            5
        );

        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "b", "c", "d", "e"].into_iter()
            )
            .digits()[0],
            5
        );
    }
}
