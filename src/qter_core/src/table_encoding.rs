use std::collections::{HashMap, HashSet};

use internment::ArcIntern;
use itertools::Itertools;

struct TableStats {
    frequencies: HashMap<ArcIntern<str>, u32>,
    length_frequencies: HashMap<usize, u32>,
    disallowed_pairs: HashSet<(ArcIntern<str>, ArcIntern<str>)>,
}

/// Returns an encoded table or None if there are too many unique generators to be able to encode them (contact Henry)
fn encode_table(algs: &[Vec<ArcIntern<str>>]) -> Option<Vec<u8>> {
    // Statistical modelling of twisty puzzle algs:
    //
    // First, we're going to keep track of frequencies of different generators. I technically don't know but I highly doubt that generators for optimal solutions will be completely uniform. Also, if Arhan decides to pick algs with better finger tricks, this will take advantage of the distribution.
    //
    // Second, we're going to keep track of the distribution of alg lengths, allowing us to set the probability of the "end of alg" symbol appropriately.
    //
    // Third, if two generators composed together equal another generator or the identity, they can never exist next to each other in an optimally solved algorithm (U U' = I, U U2 = U'). We find this list of disallowed pairs dynamically so we don't have to assume anything about notation.
    //
    // The generators are assumed to be random according to this distribution with no other patterns.

    let mut stats = algs.iter().fold(
        TableStats {
            frequencies: HashMap::new(),
            length_frequencies: HashMap::new(),
            disallowed_pairs: HashSet::new(),
        },
        |mut stats, alg| {
            *stats.length_frequencies.entry(alg.len()).or_insert(0) += 1;

            for generator in alg {
                *stats
                    .frequencies
                    .entry(ArcIntern::clone(generator))
                    .or_insert(0) += 1;
            }

            // Note: `disallowed_pairs` will actually contain the set of allowed pairs and we will take the complement of the set later
            for (a, b) in alg.iter().tuple_windows() {
                let a = ArcIntern::clone(a);
                let b = ArcIntern::clone(b);

                if a < b {
                    stats.disallowed_pairs.insert((a, b));
                } else {
                    stats.disallowed_pairs.insert((b, a));
                }
            }

            stats
        },
    );

    if stats.frequencies.len() > (1 << N) - 1 {
        return None;
    }

    let mut disallowed_pairs = HashSet::new();

    for pair in stats
        .frequencies
        .keys()
        .cartesian_product(stats.frequencies.keys())
    {
        if pair.1 > pair.0 {
            continue;
        }

        let pair = (ArcIntern::clone(pair.0), ArcIntern::clone(pair.1));

        if !stats.disallowed_pairs.contains(&pair) {
            disallowed_pairs.insert(pair);
        }
    }

    stats.disallowed_pairs = disallowed_pairs;

    // Now `disallowed_pairs` means the correct thing

    let mut stream = Vec::new();

    let mut symbol_indices = HashMap::new();

    stream.extend_from_slice(&(stats.frequencies.len() as u32).to_le_bytes());

    for (i, (symbol, freq)) in stats.frequencies.iter().enumerate() {
        symbol_indices.insert(ArcIntern::clone(symbol), i as State);
        stream.extend_from_slice(&(symbol.len() as u32).to_le_bytes());
        stream.extend_from_slice(symbol.as_bytes());
        stream.extend_from_slice(&freq.to_le_bytes());
    }

    stream.extend_from_slice(&(stats.length_frequencies.len() as u32).to_le_bytes());

    for (len, freq) in &stats.length_frequencies {
        stream.extend_from_slice(&len.to_le_bytes());
        stream.extend_from_slice(&freq.to_le_bytes());
    }

    stream.extend_from_slice(&(stats.disallowed_pairs.len() as u32).to_le_bytes());

    let dist = unweighted_ranges(stats.frequencies.len());

    let mut disallowed_pair_symbols = Vec::new();

    for pair in &stats.disallowed_pairs {
        disallowed_pair_symbols.push(*symbol_indices.get(&pair.0).unwrap());
        disallowed_pair_symbols.push(*symbol_indices.get(&pair.1).unwrap());
    }

    ans_encode(&mut stream, &disallowed_pair_symbols, |_| dist.to_owned());

    let end_of_alg_symbol = symbol_indices.len() as u16;

    let mut symbols = Vec::new();

    for (i, alg) in algs.iter().enumerate() {
        if i != 0 {
            symbols.push(end_of_alg_symbol);
        }

        for generator in alg {
            symbols.push(*symbol_indices.get(generator).unwrap());
        }
    }

    ans_encode(&mut stream, &symbols, mk_distribution_closure(stats));

    Some(stream)
}

fn unweighted_ranges(generator_count: usize) -> Vec<u16> {
    let mut dist = vec![1_u16; generator_count];
    let mut range_left = (1 << N) - generator_count as u32;

    for (i, dist_spot) in dist.iter_mut().enumerate() {
        let range_to_take = range_left / (generator_count - i) as u32;
        range_left -= range_to_take;
        *dist_spot += range_to_take as u16;
    }

    dist
}

/// Decodes a table and returns None if it can't be decoded
fn decode_table(mut data: &[u8]) -> Option<Vec<Vec<ArcIntern<str>>>> {
    let (symbol_count, new_data) = data.split_first_chunk::<4>()?;
    data = new_data;

    let mut symbols = Vec::new();
    let mut frequencies = HashMap::new();

    for _ in 0..u32::from_le_bytes(*symbol_count) {
        let (symbol_len, new_data) = data.split_first_chunk::<4>()?;
        data = new_data;
        let (generator, new_data) = data.split_at(u32::from_le_bytes(*symbol_len) as usize);
        data = new_data;

        let generator = ArcIntern::<str>::from(String::from_utf8(generator.to_owned()).ok()?);
        symbols.push(ArcIntern::clone(&generator));

        let (freq, new_data) = data.split_first_chunk::<4>()?;
        data = new_data;
        frequencies.insert(generator, u32::from_le_bytes(*freq));
    }

    let mut length_frequencies = HashMap::new();

    let (length_count, new_data) = data.split_first_chunk::<4>()?;
    data = new_data;

    for _ in 0..u32::from_le_bytes(*length_count) {
        let (length, new_data) = data.split_first_chunk::<4>()?;
        data = new_data;
        let (freq, new_data) = data.split_first_chunk::<4>()?;
        data = new_data;

        length_frequencies.insert(
            u32::from_le_bytes(*length) as usize,
            u32::from_le_bytes(*freq),
        );
    }

    let (disallowed_pair_count, new_data) = data.split_first_chunk::<4>()?;
    data = new_data;

    let dist = unweighted_ranges(frequencies.len());

    let (disallowed_pairs_symbols, taken) = ans_decode(
        data,
        Some(u32::from_le_bytes(*disallowed_pair_count) as usize),
        |_| dist.to_owned(),
    );
    data = data.split_at(taken).1;

    let mut disallowed_pairs = HashSet::new();

    for (a, b) in disallowed_pairs_symbols.iter().tuples() {
        disallowed_pairs.insert((
            ArcIntern::clone(&symbols[*a as usize]),
            ArcIntern::clone(&symbols[*b as usize]),
        ));
    }

    let stats = TableStats {
        frequencies,
        length_frequencies,
        disallowed_pairs,
    };

    let end_of_alg_symbol = stats.frequencies.len() as u16;

    let algs = ans_decode(data, None, mk_distribution_closure(stats))
        .0
        .split(|s| *s == end_of_alg_symbol)
        .map(|alg| {
            alg.iter()
                .map(|s| ArcIntern::clone(&symbols[*s as usize]))
                .collect_vec()
        })
        .collect_vec();

    Some(algs)
}

fn mk_distribution_closure(stats: TableStats) -> impl Fn(&[u16]) -> Vec<u16> {
    |_| todo!()
}

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

fn ans_encode(stream: &mut Vec<u8>, mut symbols: &[u16], next_ranges: impl Fn(&[u16]) -> Vec<u16>) {
    let ranges = next_ranges(symbols.split_last().map(|v| v.1).unwrap_or(&[]));

    let mut state = match symbols.last() {
        Some(last) => (0..State::MAX)
            .find(|i| {
                coding_function(*i, *last, &ranges).is_some_and(|v| v > StateNextDown::MAX as State)
            })
            .unwrap(),
        None => 0,
    };

    let starts_at = stream.len();

    while let Some((symbol, prev)) = symbols.split_last() {
        let ranges = next_ranges(prev);

        loop {
            match coding_function(state, *symbol, &ranges) {
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

fn ans_decode(
    data: &[u8],
    max_symbols: Option<usize>,
    next_ranges: impl Fn(&[u16]) -> Vec<u16>,
) -> (Vec<u16>, usize) {
    let len_before = data.len();

    let mut ranges = next_ranges(&[]);

    let (state, mut data) = data
        .split_first_chunk::<{ (State::BITS / 8) as usize }>()
        .unwrap();

    let mut state = State::from_le_bytes(*state);
    let mut output = Vec::new();

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

        if let Some(max) = max_symbols {
            if output.len() == max {
                break;
            }
        }

        state = ranges[s as usize] * (state >> N) + (state & mask) - cdf_val;

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

        ranges = next_ranges(&output);
    }

    (output, len_before - data.len())
}

#[cfg(test)]
mod tests {
    use crate::table_encoding::ans_decode;

    use super::ans_encode;

    #[test]
    fn test_encoding() {
        let v = [
            0, 1, 0, 2, 0, 2, 1, 0, 1, 0, 2, 0, 2, 0, 1, 2, 0, 2, 0, 1, 0, 1, 2, 0,
        ];

        let dist = |found: &[u16]| match found.last() {
            Some(prev) => {
                if *prev == 0 {
                    vec![0, 2, 2]
                } else if *prev == 1 {
                    vec![3, 0, 1]
                } else if *prev == 2 {
                    vec![3, 1, 0]
                } else {
                    panic!("{prev}");
                }
            }
            None => {
                vec![2, 1, 1]
            }
        };

        let mut encoded = Vec::new();
        ans_encode(&mut encoded, &v, dist);
        println!("{encoded:?}");
        let (decoded, taken) = ans_decode(&encoded, None, dist);
        assert_eq!(taken, 4);
        assert_eq!(decoded, v);
        encoded.extend_from_slice(&[1, 2, 3, 4, 5]);
        let (decoded, taken) = ans_decode(&encoded, Some(v.len()), dist);
        assert_eq!(taken, 4);
        assert_eq!(decoded, v);
    }
}
