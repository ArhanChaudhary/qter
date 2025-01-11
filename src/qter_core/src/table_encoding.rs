use itertools::{EitherOrBoth, Itertools};

const N: u32 = 2;

type State = u16;
type StateNextUp = u32;
type StateNextDown = u8;

fn coding_function(state: State, symbol: u16, ranges: &[u16]) -> Option<State> {
    let p = ranges[symbol as usize] as StateNextUp;
    (((state as StateNextUp / p) << N)
        + (state as StateNextUp % p)
        + ranges
            .iter()
            .take(symbol as usize)
            .map(|v| *v as StateNextUp)
            .sum::<u32>())
    .try_into()
    .ok()
}

fn ans_encode(
    symbols: &[u16],
    first_ranges: Vec<u16>,
    next_ranges: impl Fn(u16) -> Vec<u16>,
) -> Vec<u8> {
    let mut ranges = match symbols.iter().rev().nth(1) {
        Some(v) => next_ranges(*v),
        None => first_ranges.to_owned(),
    };

    let mut state = match symbols.last() {
        Some(last) => (0..State::MAX)
            .find(|i| {
                coding_function(*i, *last, &ranges).is_some_and(|v| v > StateNextDown::MAX as State)
            })
            .unwrap(),
        None => 0,
    };
    let mut stream = Vec::new();

    println!("{state}");

    for step in symbols
        .iter()
        .rev()
        .zip_longest(symbols.iter().rev().skip(1))
    {
        let (symbol, ranges) = match step {
            EitherOrBoth::Both(symbol, prev) => (symbol, next_ranges(*prev)),
            EitherOrBoth::Left(symbol) => (symbol, first_ranges.to_owned()),
            EitherOrBoth::Right(_) => unreachable!(),
        };

        loop {
            match coding_function(state, *symbol, &ranges) {
                Some(new_state) => {
                    state = new_state;
                    break;
                }
                None => {
                    stream.extend_from_slice(
                        &((state & StateNextDown::MAX as State) as StateNextDown).to_le_bytes(),
                    );
                    state >>= StateNextDown::BITS;
                }
            };
        }

        println!("{state}");
    }

    stream.extend_from_slice(&state.to_le_bytes());

    stream
}

fn ans_decode(
    data: &[u8],
    first_ranges: Vec<u16>,
    next_ranges: impl Fn(u16) -> Vec<u16>,
) -> Vec<u16> {
    let mut ranges = first_ranges;

    let (mut data, state) = data
        .split_last_chunk::<{ (State::BITS / 8) as usize }>()
        .unwrap();

    let mut state = State::from_le_bytes(*state);
    let mut output = Vec::new();

    println!("{state}");

    let mask = (1 << N) - 1;

    'decoding: loop {
        let range_spot = state & mask;

        let mut cdf_val = 0;
        let s = ranges
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

        output.push(s);

        state = ranges[s as usize] * (state >> N) + (state & mask) - cdf_val;

        while state.ilog2() < StateNextDown::BITS {
            match data.split_last_chunk::<{ (StateNextDown::BITS / 8) as usize }>() {
                Some((new_data, v)) => {
                    data = new_data;

                    state <<= StateNextDown::BITS;
                    state |= StateNextDown::from_le_bytes(*v) as State;
                }
                None => break 'decoding,
            }
        }
        println!("{state}");

        ranges = next_ranges(s);
    }

    output
}

#[cfg(test)]
mod tests {
    use crate::table_encoding::ans_decode;

    use super::ans_encode;

    #[test]
    fn bruh() {
        let v = vec![
            0, 1, 0, 2, 0, 2, 1, 0, 1, 0, 2, 0, 2, 0, 1, 2, 0, 2, 0, 1, 0, 1, 2, 0,
        ];

        let dist = |prev| {
            if prev == 0 {
                vec![0, 2, 2]
            } else if prev == 1 {
                vec![3, 0, 1]
            } else if prev == 2 {
                vec![3, 1, 0]
            } else {
                panic!("{prev}");
            }
        };

        let encoded = ans_encode(&v, vec![2, 1, 1], dist);
        let decoded = ans_decode(&encoded, vec![2, 1, 1], dist);

        assert_eq!(decoded, v);
    }
}
