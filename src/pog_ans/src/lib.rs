use std::iter;

type State = u16;
type StateNextDown = u8;

pub const N: u32 = StateNextDown::BITS;

fn coding_function(state: State, symbol: u16, ranges: &[u16]) -> Option<State> {
    let p = ranges[symbol as usize];

    if p == 0 {
        panic!("Got a symbol with range zero: {symbol}; {ranges:?}");
    }

    let divided = state / p;

    // Doing the bitshift will make the number fall out of range
    if divided.leading_zeros() < N {
        return None;
    }

    Some((divided << N) + (state % p) + ranges.iter().take(symbol as usize).copied().sum::<State>())
}

pub fn ans_encode(
    stream: &mut Vec<u8>,
    mut symbols: &[u16],
    symbol_count: usize,
    mut next_ranges: impl FnMut(Option<u16>, &mut [u16]),
) {
    let mut ranges = vec![0; symbol_count * symbols.len()];

    (iter::once(None).chain(symbols.iter().copied().map(Some)))
        .zip(ranges.chunks_mut(symbol_count))
        .for_each(|(symbol, ranges)| {
            next_ranges(symbol, ranges);
            assert!(
                ranges.iter().copied().sum::<u16>() == 1 << N,
                "Ranges must sum to {}, got {ranges:?}",
                1 << N
            )
        });

    let mut state = match symbols.last() {
        Some(last) => (0..State::MAX)
            .find(|i| {
                coding_function(*i, *last, &ranges[ranges.len() - symbol_count..])
                    .is_some_and(|v| v > StateNextDown::MAX as State)
            })
            .unwrap(),
        None => 0,
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
                        &((state & StateNextDown::MAX as State) as StateNextDown).to_be_bytes(),
                    );
                    state >>= StateNextDown::BITS;
                }
            };
        }

        symbols = prev;
    }

    stream.extend_from_slice(&state.to_be_bytes());

    stream[starts_at..].reverse();
}

pub fn ans_decode(
    data: &[u8],
    max_symbols: Option<usize>,
    symbol_count: usize,
    mut next_ranges: impl FnMut(Option<u16>, &mut [u16]),
) -> Option<(Vec<u16>, usize)> {
    if let Some(max) = max_symbols {
        if max == 0 {
            return Some((vec![], 0));
        }
    }

    let len_before = data.len();

    let mut ranges = vec![0; symbol_count];

    next_ranges(None, &mut ranges);

    let (state, mut data) = data.split_first_chunk::<{ (State::BITS / 8) as usize }>()?;

    let mut state = State::from_le_bytes(*state);
    let mut output = Vec::new();

    let mask = (1 << N) - 1;

    'decoding: loop {
        let range_spot = state & mask;

        let mut cdf_val = 0;
        let symbol = ranges
            .iter()
            .copied()
            .take_while(|v| {
                if cdf_val + v > range_spot {
                    return false;
                }

                cdf_val += v;

                true
            })
            .count() as State;

        output.push(symbol);

        if let Some(max) = max_symbols {
            if output.len() == max {
                break;
            }
        }

        state = ranges[symbol as usize] * (state >> N) + (state & mask) - cdf_val;

        while state == 0 || state.ilog2() < StateNextDown::BITS {
            if let Some((v, new_data)) =
                data.split_first_chunk::<{ (StateNextDown::BITS / 8) as usize }>()
            {
                data = new_data;

                state <<= StateNextDown::BITS;
                state |= StateNextDown::from_le_bytes(*v) as State;
            } else {
                break 'decoding;
            }
        }

        next_ranges(Some(symbol), &mut ranges);
    }

    Some((output, len_before - data.len()))
}

#[cfg(test)]
mod tests {
    use crate::{N, ans_decode, ans_encode};

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

            out.iter_mut().for_each(|v| *v *= (1 << N) / 4);
        };

        let mut encoded = Vec::new();
        ans_encode(&mut encoded, &v, 3, dist);
        println!("{encoded:?}");
        let (decoded, taken) = ans_decode(&encoded, None, 3, dist).unwrap();
        assert_eq!(taken, 4);
        assert_eq!(decoded, v);
        encoded.extend_from_slice(&[1, 2, 3, 4, 5]);
        let (decoded, taken) = ans_decode(&encoded, Some(v.len()), 3, dist).unwrap();
        assert_eq!(taken, 4);
        assert_eq!(decoded, v);
    }
}
