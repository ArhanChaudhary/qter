use bnum::{
    cast::As,
    types::{I512, U512},
};

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

pub fn lcm(a: U512, b: U512) -> U512 {
    assert_ne!(a, U512::ZERO);
    assert_ne!(b, U512::ZERO);

    b / gcd(a, b) * a
}

pub fn lcm_iter(values: impl Iterator<Item = U512>) -> U512 {
    values.fold(U512::ONE, lcm)
}

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
pub fn chinese_remainder_theorem(values: Vec<(U512, U512)>) -> U512 {
    let mut conditions = values.into_iter();

    let mut initial = match conditions.next() {
        Some(condition) => condition,
        None => return U512::ZERO,
    };

    for condition in conditions {
        let (coeffs, gcd) = extended_euclid(initial.1, condition.1);

        let diff = if condition.0 > initial.0 {
            condition.0 - initial.0
        } else {
            initial.0 - condition.0
        };

        if diff.rem_euclid(gcd) != U512::ZERO {
            panic!("Inconsistent remainders!");
        }

        let λ = diff / gcd;

        let x = if condition.0 > initial.0 {
            condition.0.as_::<I512>() - condition.1.as_::<I512>() * coeffs.1 * λ.as_::<I512>()
        } else {
            initial.0.as_::<I512>() - initial.1.as_::<I512>() * coeffs.0 * λ.as_::<I512>()
        };

        let new_modulus = lcm(initial.1, condition.1);

        initial = (x.rem_euclid(new_modulus.as_()).as_(), new_modulus);
    }

    initial.0
}

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
    use itertools::Itertools;

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

    fn _crt(v: impl IntoIterator<Item = (u64, u64)>) -> u64 {
        chinese_remainder_theorem(
            v.into_iter()
                .map(|(a, b)| (U512::from_digit(a), U512::from_digit(b)))
                .collect_vec(),
        )
        .digits()[0]
    }

    #[test]
    fn crt() {
        assert_eq!(_crt([(2, 3), (1, 2)]), 5);
        assert_eq!(_crt([(3, 4), (1, 2)]), 3);
        assert_eq!(_crt([(3, 4), (1, 2), (3, 5), (4, 7)]), 123);
    }

    #[test]
    #[should_panic(expected = "Inconsistent remainders!")]
    fn crt_evil() {
        _crt([(2, 4), (1, 2)]);
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
