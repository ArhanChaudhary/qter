use std::collections::{HashMap, HashSet};

use internment::ArcIntern;
use itertools::Itertools;
use pog_ans::{TakeFrom, ans_decode, ans_encode};

#[derive(Debug)]
struct TableStats {
    frequencies: Vec<u32>,
    length_frequencies: HashMap<usize, u32>,
    disallowed_pairs: HashSet<(usize, usize)>,
}

/// Returns an encoded table or None if there are too many unique generators to be able to encode them (contact Henry)
///
/// Also returns the compressed size of the data with the header size subtracted out.
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
            frequencies: Vec::new(),
            length_frequencies: HashMap::new(),
            disallowed_pairs: HashSet::new(),
        },
        |mut stats, alg| {
            *stats.length_frequencies.entry(alg.len()).or_insert(0) += 1;

            for generator in alg {
                let idx = match symbol_indices.get(generator) {
                    None => {
                        let idx = symbol_indices.len();
                        symbol_indices.insert(ArcIntern::clone(generator), idx);
                        stats.frequencies.push(0);
                        idx
                    }
                    Some(&idx) => idx,
                };

                stats.frequencies[idx] += 1;
            }

            // Note: `disallowed_pairs` will actually contain the set of allowed pairs and we will take the complement of the set later
            for (a, b) in alg.iter().tuple_windows() {
                let a = *symbol_indices.get(a).unwrap();
                let b = *symbol_indices.get(b).unwrap();

                if a < b {
                    stats.disallowed_pairs.insert((a, b));
                } else {
                    stats.disallowed_pairs.insert((b, a));
                }
            }

            stats
        },
    );

    if stats.frequencies.len() > (1 << u8::BITS) - 1 {
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

    for (symbol, &idx) in symbol_indices.iter().sorted_unstable_by_key(|(_, i)| **i) {
        let freq = stats.frequencies[idx];

        stream.extend_from_slice(&(symbol.len() as u32).to_le_bytes());
        stream.extend_from_slice(symbol.as_bytes());
        stream.extend_from_slice(&freq.to_le_bytes());
    }

    stream.extend_from_slice(&(stats.length_frequencies.len() as u32).to_le_bytes());

    for (len, freq) in &stats.length_frequencies {
        stream.extend_from_slice(&(*len as u32).to_le_bytes());
        stream.extend_from_slice(&freq.to_le_bytes());
    }

    let mut disallowed_pair_table = HashMap::new();

    for (a, b) in &stats.disallowed_pairs {
        disallowed_pair_table.entry(a).or_insert(Vec::new()).push(b);
    }

    let end_of_alg_symbol = symbol_indices.len();

    let mut disallowed_pair_symbols = Vec::new();

    for (i, mut entry) in disallowed_pair_table
        .into_iter()
        .sorted_unstable_by_key(|entry| entry.0)
        .enumerate()
    {
        if i != 0 {
            disallowed_pair_symbols.push(end_of_alg_symbol);
        }

        entry.1.sort_unstable();

        disallowed_pair_symbols.push(*entry.0);
        for item in entry.1 {
            disallowed_pair_symbols.push(*item);
        }
    }

    stream.extend_from_slice(&(disallowed_pair_symbols.len() as u32).to_le_bytes());

    ans_encode(
        &mut stream,
        &disallowed_pair_symbols,
        stats.frequencies.len() + 1,
        disallowed_pair_symbols_distribution_closure(),
    );

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

    ans_encode(
        &mut stream,
        &symbols,
        stats.frequencies.len() + 1,
        mk_distribution_closure(stats),
    );

    let after = stream.len();

    Some((stream, after - before))
}

fn rest_unweighted(ranges: &mut [u16], mut range_left: usize) {
    ranges.iter_mut().for_each(|v| {
        if *v != 0 {
            *v = 1
        }
    });

    let mut amt_to_set = ranges.iter().filter(|v| **v != 0).count();

    for dist_spot in ranges.iter_mut() {
        if *dist_spot == 0 {
            continue;
        }

        let range_to_take = ((range_left + amt_to_set) / amt_to_set).saturating_sub(1);
        range_left -= range_to_take;
        *dist_spot += range_to_take as u16;
        amt_to_set -= 1;
    }
}

fn rest_weighted(ranges: &mut [u16], mut range_left: usize, distribution: &[u32]) {
    let mut total_weight = 0;
    let mut amt_to_set = 0;

    ranges.iter_mut().enumerate().for_each(|(i, v)| {
        if *v != 0 {
            *v = 1;

            total_weight += distribution[i] as usize;
            amt_to_set += 1;
        }
    });

    for (i, dist_spot) in ranges
        .iter_mut()
        .enumerate()
        .sorted_unstable_by_key(|(i, _)| distribution[*i])
    {
        if *dist_spot == 0 {
            continue;
        }

        let range_available = range_left + amt_to_set;

        let range_to_take =
            (range_available * distribution[i] as usize / total_weight).saturating_sub(1);
        range_left -= range_to_take;
        *dist_spot += range_to_take as u16;
        total_weight -= distribution[i] as usize;
        amt_to_set -= 1;
    }
}

/// Decodes a table and returns None if it can't be decoded
pub fn decode_table(data: &mut impl Iterator<Item = u8>) -> Option<Vec<Vec<ArcIntern<str>>>> {
    let symbol_count = u32::take_from(data)?;

    let mut symbols = Vec::new();
    let mut frequencies = Vec::new();

    for _ in 0..symbol_count {
        let symbol_len = u32::take_from(data)?;
        let generator = data.take(symbol_len as usize).collect_vec();

        let generator = ArcIntern::<str>::from(String::from_utf8(generator).ok()?);
        symbols.push(ArcIntern::clone(&generator));

        frequencies.push(u32::take_from(data)?);
    }

    let mut length_frequencies = HashMap::new();

    let length_count = u32::take_from(data)?;

    for _ in 0..length_count {
        length_frequencies.insert(u32::take_from(data)? as usize, u32::take_from(data)?);
    }

    let disallowed_pair_count = u32::take_from(data)? as usize;

    let disallowed_pairs_symbols = ans_decode(
        data,
        Some(disallowed_pair_count),
        frequencies.len() + 1,
        disallowed_pair_symbols_distribution_closure(),
    )?;

    let end_of_alg_symbol = frequencies.len();

    let mut disallowed_pairs = HashSet::new();

    for (item_a, disallowed_with) in disallowed_pairs_symbols.iter().batching(|v| {
        let item = v.next()?;

        let mut disallowed_with = vec![];

        for v in v.by_ref() {
            if *v == end_of_alg_symbol {
                break;
            }

            disallowed_with.push(v);
        }

        Some((item, disallowed_with))
    }) {
        for item_b in disallowed_with {
            disallowed_pairs.insert((*item_a, *item_b));
        }
    }

    let stats = TableStats {
        frequencies,
        length_frequencies,
        disallowed_pairs,
    };

    let algs = ans_decode(
        data,
        None,
        stats.frequencies.len() + 1,
        mk_distribution_closure(stats),
    )?
    .split(|s| *s == end_of_alg_symbol)
    .map(|alg| {
        alg.iter()
            .map(|s| ArcIntern::clone(&symbols[*s]))
            .collect_vec()
    })
    .collect_vec();

    Some(algs)
}

fn disallowed_pair_symbols_distribution_closure() -> impl FnMut(Option<usize>, &mut [u16]) {
    let mut min_key_seeable = 0;
    let mut prev_end_of_alg = false;

    move |found, out| {
        let end_of_alg_symbol = out.len() - 1;

        if prev_end_of_alg {
            min_key_seeable = found.unwrap();
            out[end_of_alg_symbol] = 0;
        } else {
            out[end_of_alg_symbol] = 1;
        }

        if found == Some(end_of_alg_symbol) {
            min_key_seeable += 1;
        }

        let mut min_num_seeable = min_key_seeable;

        if let Some(found) = found {
            if found != end_of_alg_symbol {
                min_num_seeable = found + (!prev_end_of_alg) as usize;
            }
        }

        if found == Some(end_of_alg_symbol) || found.is_none() {
            prev_end_of_alg = true;
            out[end_of_alg_symbol] = 0;
        } else {
            prev_end_of_alg = false;
        }

        out[0..min_num_seeable].fill(0);
        out[min_num_seeable..end_of_alg_symbol].fill(1);

        rest_unweighted(
            out,
            (1 << u8::BITS)
                - (end_of_alg_symbol - min_num_seeable)
                - out[end_of_alg_symbol] as usize,
        );
    }
}

fn mk_distribution_closure(stats: TableStats) -> impl FnMut(Option<usize>, &mut [u16]) {
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

    let end_of_alg_symbol = generator_count;
    let mut len = 0;

    move |prev, out| {
        if prev.is_some_and(|v| v != end_of_alg_symbol) {
            len += 1;
        } else {
            len = 0;
        }

        out.fill(0);

        let mut range_left = 1 << u8::BITS;

        if let Some((len_chance, lens_cdf)) = lens_cdf.get(&len) {
            if *lens_cdf == 0 {
                out[end_of_alg_symbol] = range_left;
                return;
            } else {
                let amt_to_give = ((range_left as u32 * *len_chance / (*len_chance + *lens_cdf))
                    as u16)
                    .min(range_left - generator_count as u16)
                    .max(1);

                out[end_of_alg_symbol] = amt_to_give;
                range_left -= amt_to_give;
            }
        }

        for (sym, spot) in out.iter_mut().enumerate().take(generator_count) {
            if let Some(prev) = prev {
                if stats.disallowed_pairs.contains(&if prev < sym {
                    (prev, sym)
                } else {
                    (sym, prev)
                }) {
                    continue;
                }
            }

            *spot = 1;
            range_left -= 1;
        }

        rest_weighted(
            &mut out[..end_of_alg_symbol],
            range_left as usize,
            &stats.frequencies,
        )
    }
}

#[cfg(test)]
mod tests {
    use internment::ArcIntern;
    use itertools::Itertools;

    use crate::table_encoding::decode_table;

    use super::encode_table;

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
        let decoded = decode_table(&mut encoded.iter().copied()).unwrap();
        assert_eq!(algs, decoded);
        // panic!()
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
        let decoded = decode_table(&mut encoded.iter().copied()).unwrap();
        assert_eq!(algs, decoded);

        // panic!(
        //     "{} → {} : {:.2}\n{} → {} : {:.2}",
        //     spec.len(),
        //     encoded.len(),
        //     1. - encoded.len() as f64 / spec.len() as f64,
        //     spec.len(),
        //     data_without_header,
        //     1. - data_without_header as f64 / spec.len() as f64
        // );
    }
}
