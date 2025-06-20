use crate::{
    I, Int, U,
    architectures::{Algorithm, Permutation},
};

/// Calculate the GCD of two numbers
#[must_use]
pub fn gcd(mut a: Int<U>, mut b: Int<U>) -> Int<U> {
    loop {
        if b.is_zero() {
            return a;
        }

        let rem = a % b;
        a = b;
        b = rem;
    }
}

/// Calculate the LCM of two numbers
///
/// # Panics
///
/// Panics if either number is zero.
#[must_use]
pub fn lcm(a: Int<U>, b: Int<U>) -> Int<U> {
    assert!(!a.is_zero());
    assert!(!b.is_zero());

    b / gcd(a, b) * a
}

/// Calculate the LCM of a list of numbers
pub fn lcm_iter(values: impl Iterator<Item = Int<U>>) -> Int<U> {
    values.fold(Int::one(), lcm)
}

/// Calculate the GCD of two numbers as well as the coefficients of Bézout's identity
#[must_use]
pub fn extended_euclid(mut a: Int<U>, mut b: Int<U>) -> ((Int<I>, Int<I>), Int<U>) {
    let mut a_coeffs = (Int::<I>::one(), Int::<I>::zero());
    let mut b_coeffs = (Int::<I>::zero(), Int::<I>::one());

    loop {
        if b.is_zero() {
            return (a_coeffs, a);
        }

        let to_sub = a / b;
        let new_coeffs = (
            a_coeffs.0 - b_coeffs.0 * to_sub,
            a_coeffs.1 - b_coeffs.1 * to_sub,
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
    mut conditions: impl Iterator<Item = Option<(Int<U>, Int<U>)>>,
) -> Option<Int<U>> {
    let (mut prev_remainder, mut prev_modulus) = match conditions.next() {
        Some(Some(condition)) => condition,
        Some(None) => return None,
        None => return Some(Int::<U>::zero()),
    };

    for cond in conditions {
        let (remainder, modulus) = cond?;

        let (coeffs, gcd) = extended_euclid(prev_modulus, modulus);

        let diff = if remainder > prev_remainder {
            remainder - prev_remainder
        } else {
            prev_remainder - remainder
        };

        if !(diff % gcd).is_zero() {
            return None;
        }

        let λ = diff / gcd;

        let x = if remainder > prev_remainder {
            remainder - modulus * coeffs.1 * λ
        } else {
            prev_remainder - prev_modulus * coeffs.0 * λ
        };

        let new_modulus = lcm(prev_modulus, modulus);

        prev_remainder = x % new_modulus;
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
) -> usize {
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

    current_repeat_length
}

/// Decode the permutation using the register generator and the given facelets.
///
/// In general, an arbitrary scramble cannot be decoded. If this is the case, the function will return `None`.
pub fn decode(
    permutation: &Permutation,
    facelets: &[usize],
    generator: &Algorithm,
) -> Option<Int<U>> {
    chinese_remainder_theorem(facelets.iter().map(|&facelet| {
        let maps_to = permutation.mapping()[facelet];

        let chromatic_order = generator.chromatic_orders_by_facelets()[facelet];

        if maps_to == facelet {
            return Some((Int::zero(), chromatic_order));
        }

        let mut i = Int::<U>::one();
        let mut maps_to_found_at = None;
        let mut facelet_at = generator.permutation().mapping()[facelet];

        while facelet_at != facelet {
            if facelet_at == maps_to {
                maps_to_found_at = Some(i);
                break;
            }

            facelet_at = generator.permutation().mapping()[facelet_at];
            i += Int::<U>::one();
        }

        maps_to_found_at.map(|found_at| (found_at % chromatic_order, chromatic_order))
    }))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chumsky::Parser;
    use internment::ArcIntern;

    use crate::{
        File, Int, U,
        architectures::{Algorithm, puzzle_definition},
        discrete_math::{
            decode, extended_euclid, gcd, lcm,
            length_of_substring_that_this_string_is_n_repeated_copies_of,
        },
    };

    use super::chinese_remainder_theorem;

    #[test]
    fn lcm_and_gcd() {
        let lcm_int = |a: u64, b: u64| lcm(Int::from(a), Int::from(b)).to_u64();
        let gcd_int = |a: u64, b: u64| gcd(Int::from(a), Int::from(b)).to_u64();
        let extended_euclid_int = |a: u64, b: u64| {
            let ((x, y), z) = extended_euclid(Int::from(a), Int::from(b));
            assert_eq!(Int::<U>::from(a) * x + Int::<U>::from(b) * y, z);
            z.to_u64()
        };

        assert_eq!(gcd_int(3, 5), 1);
        assert_eq!(gcd_int(3, 6), 3);
        assert_eq!(gcd_int(4, 6), 2);

        assert_eq!(extended_euclid_int(3, 5), 1);
        assert_eq!(extended_euclid_int(3, 6), 3);
        assert_eq!(extended_euclid_int(4, 6), 2);

        assert_eq!(lcm_int(3, 5), 15);
        assert_eq!(lcm_int(3, 6), 6);
        assert_eq!(lcm_int(4, 6), 12);
    }

    fn crt_int(v: impl IntoIterator<Item = (u64, u64)>) -> Option<u64> {
        chinese_remainder_theorem(
            v.into_iter()
                .map(|(a, b)| Some((Int::from(a), Int::from(b)))),
        )
        .map(|v| v.to_u64())
    }

    #[test]
    fn crt() {
        assert_eq!(crt_int([(2, 3), (1, 2)]), Some(5));
        assert_eq!(crt_int([(3, 4), (1, 2)]), Some(3));
        assert_eq!(crt_int([(3, 4), (1, 2), (3, 5), (4, 7)]), Some(123));
        assert_eq!(crt_int([(2, 4), (1, 2)]), None);
    }

    #[test]
    fn length_of_substring_whatever() {
        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "a", "a", "a"].into_iter()
            ),
            1
        );

        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "b", "a", "b"].into_iter()
            ),
            2
        );

        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "b", "a", "b", "a"].into_iter()
            ),
            5
        );

        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "b", "c", "d", "e"].into_iter()
            ),
            5
        );
    }

    #[test]
    fn test_decode() {
        let cube_def = puzzle_definition().parse(File::from("3x3")).unwrap();

        let mut cube = cube_def.perm_group.identity();

        let permutation = Algorithm::new_from_move_seq(
            Arc::clone(&cube_def.perm_group),
            vec![ArcIntern::from("U")],
        )
        .unwrap();

        assert_eq!(decode(&cube, &[8], &permutation).unwrap(), Int::<U>::zero());

        cube.compose_into(permutation.permutation());
        assert_eq!(decode(&cube, &[8], &permutation).unwrap(), Int::<U>::one());

        cube.compose_into(permutation.permutation());
        assert_eq!(decode(&cube, &[8], &permutation).unwrap(), Int::from(2));

        cube.compose_into(permutation.permutation());
        assert_eq!(decode(&cube, &[8], &permutation).unwrap(), Int::from(3));

        cube.compose_into(permutation.permutation());
        assert_eq!(decode(&cube, &[8], &permutation).unwrap(), Int::from(0));
    }
}
