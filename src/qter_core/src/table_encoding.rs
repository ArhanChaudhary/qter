use std::collections::{HashMap, HashSet};

use internment::ArcIntern;
use itertools::Itertools;

struct TableStats {
    frequencies: HashMap<ArcIntern<str>, u32>,
    length_frequencies: HashMap<usize, u32>,
    disallowed_pairs: HashSet<(u16, u16)>,
}

/// Returns an encoded table or None if there are too many unique generators to be able to encode them (contact Henry)
pub fn encode_table(algs: &[Vec<ArcIntern<str>>]) -> Option<(Vec<u8>, usize)> {
    // Statistical modelling of twisty puzzle algs:
    //
    // First, we're going to keep track of frequencies of different generators. I technically don't know but I highly doubt that generators for optimal solutions will be completely uniform. Also, if Arhan decides to pick algs with better finger tricks, this will take advantage of the distribution.
    //
    // Second, we're going to keep track of the distribution of alg lengths, allowing us to set the probability of the "end of alg" symbol appropriately.
    //
    // Third, if two generators composed together equal another generator or the identity, they can never exist next to each other in an optimally solved algorithm (U U' = I, U U2 = U'). We find this list of disallowed pairs dynamically so we don't have to assume anything about notation. This list of disallowed pairs is assumed to be sparse.
    //
    // The generators are assumed to be random according to this distribution with no other patterns.

    let mut symbol_indices = HashMap::new();

    let mut stats = algs.iter().fold(
        TableStats {
            frequencies: HashMap::new(),
            length_frequencies: HashMap::new(),
            disallowed_pairs: HashSet::new(),
        },
        |mut stats, alg| {
            *stats.length_frequencies.entry(alg.len()).or_insert(0) += 1;

            for generator in alg {
                if !symbol_indices.contains_key(generator) {
                    let idx = symbol_indices.len() as u16;
                    symbol_indices.insert(ArcIntern::clone(generator), idx);
                }

                *stats
                    .frequencies
                    .entry(ArcIntern::clone(generator))
                    .or_insert(0) += 1;
            }

            // Note: `disallowed_pairs` will actually contain the set of allowed pairs and we will take the complement of the set later
            for (a, b) in alg.iter().tuple_windows() {
                let a = symbol_indices.get(a).unwrap();
                let b = symbol_indices.get(b).unwrap();

                if a < b {
                    stats.disallowed_pairs.insert((*a, *b));
                } else {
                    stats.disallowed_pairs.insert((*b, *a));
                }
            }

            stats
        },
    );

    if stats.frequencies.len() > (1 << N) - 1 {
        return None;
    }

    let mut disallowed_pairs = HashSet::new();

    for pair in symbol_indices
        .values()
        .cartesian_product(symbol_indices.values())
    {
        if pair.1 < pair.0 {
            continue;
        }

        let pair = (*pair.0, *pair.1);

        if !stats.disallowed_pairs.contains(&pair) {
            disallowed_pairs.insert(pair);
        }
    }

    stats.disallowed_pairs = disallowed_pairs;

    // Now `disallowed_pairs` means the correct thing

    let mut stream = Vec::new();

    stream.extend_from_slice(&(stats.frequencies.len() as u32).to_le_bytes());

    for (symbol, _) in symbol_indices.iter().sorted_unstable_by_key(|(_, i)| **i) {
        let freq = stats.frequencies.get(symbol).unwrap();

        stream.extend_from_slice(&(symbol.len() as u32).to_le_bytes());
        stream.extend_from_slice(symbol.as_bytes());
        stream.extend_from_slice(&freq.to_le_bytes());
    }

    stream.extend_from_slice(&(stats.length_frequencies.len() as u32).to_le_bytes());

    for (len, freq) in &stats.length_frequencies {
        stream.extend_from_slice(&(*len as u32).to_le_bytes());
        stream.extend_from_slice(&freq.to_le_bytes());
    }

    stream.extend_from_slice(&(stats.disallowed_pairs.len() as u32 * 2).to_le_bytes());

    let dist = unweighted_ranges(stats.frequencies.len());

    let mut disallowed_pair_symbols = Vec::new();

    for pair in &stats.disallowed_pairs {
        disallowed_pair_symbols.push(pair.0);
        disallowed_pair_symbols.push(pair.1);
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

    let before = stream.len();

    ans_encode(&mut stream, &symbols, mk_distribution_closure(stats));

    let after = stream.len();

    Some((stream, after - before))
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
pub fn decode_table(mut data: &[u8]) -> Option<Vec<Vec<ArcIntern<str>>>> {
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
    let disallowed_pair_count = u32::from_le_bytes(*disallowed_pair_count) as usize;

    let dist = unweighted_ranges(frequencies.len());

    let (disallowed_pairs_symbols, taken) =
        ans_decode(data, Some(disallowed_pair_count), |_| dist.to_owned())?;
    data = data.split_at(taken).1;

    let mut disallowed_pairs = HashSet::new();

    for (a, b) in disallowed_pairs_symbols.iter().tuples() {
        disallowed_pairs.insert((*a, *b));
    }

    let stats = TableStats {
        frequencies,
        length_frequencies,
        disallowed_pairs,
    };

    let end_of_alg_symbol = stats.frequencies.len() as u16;

    let algs = ans_decode(data, None, mk_distribution_closure(stats))?
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
    let generator_count = stats.frequencies.len();

    let mut total_lens = 0;
    let lens_cdf = stats
        .length_frequencies
        .into_iter()
        .sorted_unstable_by(|a, b| b.0.cmp(&a.0))
        .map(|v| {
            let out = (v.0, (v.1, total_lens));
            total_lens += v.1;
            out
        })
        .collect::<HashMap<_, _>>();

    println!(
        "{:?}",
        stats
            .disallowed_pairs
            .iter()
            .sorted_unstable()
            .collect_vec()
    );

    let end_of_alg_symbol = generator_count as u16;

    move |found| {
        let len = match found.iter().rposition(|v| *v == end_of_alg_symbol) {
            Some(pos) => found.len() - pos - 1,
            None => found.len(),
        };

        let mut dist = vec![0_u16; generator_count + 1];
        let mut range_left = 1 << N;

        if let Some((len_chance, lens_cdf)) = lens_cdf.get(&len) {
            if *lens_cdf == 0 {
                dist[end_of_alg_symbol as usize] = range_left;
                return dist;
            } else {
                let amt_to_give = ((range_left as u32 * *len_chance / (*len_chance + *lens_cdf))
                    as u16)
                    .min(range_left - generator_count as u16);

                dist[end_of_alg_symbol as usize] = amt_to_give;
                range_left -= amt_to_give;
            }
        }

        let mut generators_possible = 0;

        for (sym, spot) in dist
            .iter_mut()
            .enumerate()
            .map(|(i, v)| (i as u16, v))
            .take(generator_count)
        {
            if let Some(last) = found.last() {
                if stats.disallowed_pairs.contains(&if *last < sym {
                    (*last, sym)
                } else {
                    (sym, *last)
                }) {
                    continue;
                }
            }

            *spot = 1;
            range_left -= 1;
            generators_possible += 1;
        }

        for dist_spot in dist.iter_mut().take(generator_count) {
            if *dist_spot == 0 {
                continue;
            }

            let range_to_take =
                ((range_left + generators_possible) / generators_possible).saturating_sub(1);
            range_left -= range_to_take;
            *dist_spot += range_to_take;
            generators_possible -= 1;
        }

        dist
    }
}

const N: u32 = 8;

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
) -> Option<(Vec<u16>, usize)> {
    if let Some(max) = max_symbols {
        if max == 0 {
            return Some((vec![], 0));
        }
    }

    let len_before = data.len();

    let mut ranges = next_ranges(&[]);

    let (state, mut data) = data.split_first_chunk::<{ (State::BITS / 8) as usize }>()?;

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

    Some((output, len_before - data.len()))
}

#[cfg(test)]
mod tests {
    use internment::ArcIntern;
    use itertools::Itertools;

    use crate::table_encoding::{ans_decode, decode_table, N};

    use super::{ans_encode, encode_table};

    #[test]
    fn test_encoding() {
        let v = [
            0, 1, 0, 2, 0, 2, 1, 0, 1, 0, 2, 0, 2, 0, 1, 2, 0, 2, 0, 1, 0, 1, 2, 0,
        ];

        let dist = |found: &[u16]| {
            let mut dist = match found.last() {
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

            dist.iter_mut().for_each(|v| *v *= (1 << N) / 4);

            dist
        };

        let mut encoded = Vec::new();
        ans_encode(&mut encoded, &v, dist);
        println!("{encoded:?}");
        let (decoded, taken) = ans_decode(&encoded, None, dist).unwrap();
        assert_eq!(taken, 4);
        assert_eq!(decoded, v);
        encoded.extend_from_slice(&[1, 2, 3, 4, 5]);
        let (decoded, taken) = ans_decode(&encoded, Some(v.len()), dist).unwrap();
        assert_eq!(taken, 4);
        assert_eq!(decoded, v);
    }

    fn mk_algs_datastructure(spec: &str) -> Vec<Vec<ArcIntern<str>>> {
        spec.split('\n')
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|alg| {
                alg.split(' ')
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
                    .map(ArcIntern::from)
                    .collect_vec()
            })
            .collect_vec()
    }

    #[test]
    fn test_table_encoding() {
        let algs = mk_algs_datastructure(
            "
                A B C
                C B A
            ",
        );

        let encoded = encode_table(&algs).unwrap().0;
        println!("{encoded:?}");
        let decoded = decode_table(&encoded).unwrap();
        assert_eq!(algs, decoded);
    }

    #[test]
    fn extensive_table_encoding_test() {
        // All the OLL PLL algs
        let spec = "R' U L' U2 R U' R' U2 R L
R U R' F' R U R' U' R' F R2 U' R'
R U' R' U' R U R D R' U' R D' R' U2 R'
R' U2 R U2 R' F R U R' U' R' F' R2
F' R U R' U' R' F R2 F U' R' U' R U F' R'
r' D' F r U' r' F' D r2 U r' U' r' F r F'
x' R U' R' D R U R' D' R U R' D  R U' R' D' x
R' U' F' R U R' U' R' F R2 U' R' U' R U R' U R
M2' U M2' U2 M2' U M2'
R U R' U' R' F R2 U' R' U' R U R' F'
R' U R' U' R D' R' D R' U D' R2 U' R2' D R2
F R U' R' U' R U R F' R U R' U' R' F R F'
M' U M2' U M2' U M' U2 M2'
R2 U R' U R' U' R U' R2 U' D R' U R D'
R' U' R U D' R2' U R' U R U' R U' R2' D
R2 F2 R U2 R U2 R' F R U R' U' R' F R2
R U R' U' D R2 U' R U' R' U R' U R2 D'
R U R' U' R U' R' F' U' F R U R'
F R' F R2 U' R' U' R U R' F2
R U R' U R U2 R' F R U R' U' F'
R' U' R U' R' U2 R F R U R' U' F'
L F' L' U' L U F U' L'
R' F R U R' U' F' U R
R U R2 U' R' F R U R U' F'
R' U' R' F R F' U R
r U R' U' r' R U R U' R'
R U R' U' M' U R U' r'
R U2 R' U' R U R' U' R U' R'
R U2 R2 U' R2 U' R2 U2 R
R2 D' R U2 R' D R U2 R
r U R' U' r' F R F'
F' r U R' U' r' F R
R U2 R' U' R U' R'
R U R' U R U2 R'
R U2 R2 F R F' U2 R' F R F'
r U r' U2 r U2 R' U2 R U' r'
r' R2 U R' U r U2 r' U M'
M U' r U2 r' U' R U' R' M'
F R' F' R2 r' U R U' R' U' M'
r U R' U R U2 r2 U' R U' R' U2 r
r' R U R U R' U' M' R' F R F'
r U R' U' M2 U R U' R' U' M'
R U R' U' R' F R2 U R' U' F'
R U R' U R' F R F' R U2 R'
R U2 R2 F R F' R U2 R'
F R' F' R U R U' R'
F U R U' R' U R U' R' F'
R U R' U R U' B U' B' R'
R' F R U R U' R2 F' R2 U' R' U R U R'
r' U' r U' R' U R U' R' U R r' U r
F U R U' R2 F' R U R U' R'
R' F R U R' F' R F U' F'
l' U' l L' U' L U l' U l
r U r' R U R' U' r U' r'
R' U' F U R U' R' F' R
L U F' U' L' U L F L'
F' U' L' U L F
F U R U' R' F'
R' U' R' F R F' R' F R F' U R
F R U R' U' R U R' U' F'
r U' r2 U r2 U r2 U' r
r' U r2 U' r2 U' r2 U r'
l' U2 L U L' U' L U L' U l
r U2 R' U' R U R' U' R U' r'
r U R' U R U2 r'
l' U' L U' L' U2 l
r U R' U R' F R F' R U2 r'
M' R' U' R U' R' U2 R U' R r'
l' U2 L U L' U l
r U2 R' U' R U' r'
R U R' U' R' F R F'
F R U R' U' F'
L' U' L U' L' U L U L F' L' F
R U R' U R U' R' U' R' F R F'";

        let algs = mk_algs_datastructure(spec);

        let (encoded, data_without_header) = encode_table(&algs).unwrap();
        println!("{encoded:?}");
        let decoded = decode_table(&encoded).unwrap();
        assert_eq!(algs, decoded);

        panic!(
            "{} → {} : {:.2}\n{} → {} : {:.2}",
            spec.len(),
            encoded.len(),
            1. - encoded.len() as f64 / spec.len() as f64,
            spec.len(),
            data_without_header,
            1. - data_without_header as f64 / spec.len() as f64
        );
    }
}
