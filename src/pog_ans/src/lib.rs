use std::iter::{self, Sum};

use num_traits::{NumAssign, NumCast, PrimInt, ToBytes};

trait Seal {}

pub trait State: TakeFrom + core::fmt::Display + core::fmt::Debug + Sum + NumAssign {
    type NextDown: TakeFrom;
    const RANGE_SIZE: Self;
    const RANGE_BITS: usize;

    fn ilog2(self) -> u32;
}

#[allow(private_bounds)]
pub trait TakeFrom: Seal + PrimInt + ToBytes {
    fn take_from(iter: &mut impl Iterator<Item = u8>) -> Option<Self>;
}

macro_rules! take_from {
    ($ty:ty) => {
        impl Seal for $ty {}

        impl TakeFrom for $ty {
            fn take_from(iter: &mut impl Iterator<Item = u8>) -> Option<Self> {
                let mut data = [0; { Self::BITS as usize / 8 }];
                for spot in data.iter_mut() {
                    let value = iter.next()?;
                    *spot = value;
                }
                Some(Self::from_le_bytes(data))
            }
        }
    };
}

macro_rules! state {
    ($ty:ty, $next_down:ty) => {
        take_from!($ty);

        impl State for $ty {
            type NextDown = $next_down;
            const RANGE_SIZE: $ty = (<$next_down>::MAX as $ty) + 1;
            const RANGE_BITS: usize = <$next_down>::BITS as usize;

            fn ilog2(self) -> u32 {
                self.ilog2()
            }
        }
    };
}

take_from!(u8);
state!(u16, u8);
state!(u32, u16);
state!(u64, u32);
state!(u128, u64);

fn coding_function<S: State>(state: S, symbol: usize, ranges: &[S]) -> Option<S> {
    let p = ranges[symbol];

    if p == S::zero() {
        panic!("Got a symbol with range zero: {symbol}; {ranges:?}");
    }

    let divided = state / p;

    // Doing the bitshift will make the number fall out of range
    if divided.leading_zeros() < (S::RANGE_BITS as u32) {
        return None;
    }

    Some((divided << S::RANGE_BITS) + (state % p) + ranges.iter().take(symbol).copied().sum::<S>())
}

pub fn ans_encode<S: State>(
    stream: &mut Vec<u8>,
    mut symbols: &[usize],
    symbol_count: usize,
    mut next_ranges: impl FnMut(Option<usize>, &mut [S]),
) {
    let mut ranges = vec![S::zero(); symbol_count * symbols.len()];

    (iter::once(None).chain(symbols.iter().copied().map(Some)))
        .zip(ranges.chunks_mut(symbol_count))
        .for_each(|(symbol, ranges)| {
            next_ranges(symbol, ranges);
            assert!(
                ranges.iter().copied().sum::<S>() == S::RANGE_SIZE,
                "Ranges must sum to {}, got {ranges:?}",
                S::RANGE_SIZE
            )
        });

    let mut state = match symbols.last() {
        Some(last) => {
            let mut i = S::zero();
            loop {
                if coding_function(i, *last, &ranges[ranges.len() - symbol_count..])
                    .is_some_and(|v| v > S::RANGE_SIZE - S::one())
                {
                    break i;
                }
                i += S::one();

                if i > S::RANGE_SIZE {
                    panic!();
                }
            }
        }
        None => S::zero(),
    };

    let starts_at = stream.len();

    while let Some((symbol, prev)) = symbols.split_last() {
        let range = &ranges[prev.len() * symbol_count..(prev.len() + 1) * symbol_count];

        loop {
            match coding_function(state, *symbol, range) {
                Some(new_state) => {
                    state = new_state;
                    break;
                }
                None => {
                    stream.extend_from_slice(
                        (<S::NextDown as NumCast>::from(state & (S::RANGE_SIZE - S::one())))
                            .unwrap()
                            .to_be_bytes()
                            .as_ref(),
                    );
                    state = state >> S::RANGE_BITS;
                }
            };
        }

        symbols = prev;
    }

    stream.extend_from_slice(state.to_be_bytes().as_ref());

    stream[starts_at..].reverse();
}

pub fn ans_decode<S: State>(
    data: &mut impl Iterator<Item = u8>,
    max_symbols: Option<usize>,
    symbol_count: usize,
    mut next_ranges: impl FnMut(Option<usize>, &mut [S]),
) -> Option<Vec<usize>> {
    if let Some(max) = max_symbols {
        if max == 0 {
            return Some(vec![]);
        }
    }

    let mut ranges = vec![S::zero(); symbol_count];

    next_ranges(None, &mut ranges);

    let mut state = S::take_from(data)?;

    let mut output = Vec::new();

    let mask = S::RANGE_SIZE - S::one();

    'decoding: loop {
        let range_spot = state & mask;

        let mut cdf_val = S::zero();
        let symbol = ranges
            .iter()
            .copied()
            .take_while(|v| {
                if cdf_val + *v > range_spot {
                    return false;
                }

                cdf_val += *v;

                true
            })
            .count();

        output.push(symbol);

        if let Some(max) = max_symbols {
            if output.len() == max {
                break;
            }
        }

        state = ranges[symbol] * (state >> S::RANGE_BITS) + (state & mask) - cdf_val;

        while state == S::zero() || state.ilog2() < (S::RANGE_BITS as u32) {
            if let Some(v) = S::NextDown::take_from(data) {
                state = (state << S::RANGE_BITS) | S::from(v).unwrap();
            } else {
                break 'decoding;
            }
        }

        next_ranges(Some(symbol), &mut ranges);
    }

    Some(output)
}

#[cfg(test)]
mod tests {
    use crate::{ans_decode, ans_encode};

    #[test]
    fn test_encoding() {
        let v = [
            0, 1, 0, 2, 0, 2, 1, 0, 1, 0, 2, 0, 2, 0, 1, 2, 0, 2, 0, 1, 0, 1, 2, 0,
        ];

        let dist = |prev, out: &mut [u16]| {
            let dist = match prev {
                Some(prev) => {
                    if prev == 0 {
                        [0, 2, 2]
                    } else if prev == 1 {
                        [3, 0, 1]
                    } else if prev == 2 {
                        [3, 1, 0]
                    } else {
                        panic!("{prev}");
                    }
                }
                None => [2, 1, 1],
            };

            out.copy_from_slice(&dist);

            out.iter_mut().for_each(|v| *v *= (1 << 8) / 4);
        };

        let mut encoded = Vec::new();
        ans_encode(&mut encoded, &v, 3, dist);
        println!("{encoded:?}");
        let decoded = ans_decode(&mut encoded.iter().copied(), None, 3, dist).unwrap();
        assert_eq!(decoded, v);
        encoded.extend_from_slice(&[1, 2, 3, 4, 5]);
        let decoded = ans_decode(&mut encoded.iter().copied(), Some(v.len()), 3, dist).unwrap();
        assert_eq!(decoded, v);
    }
}
