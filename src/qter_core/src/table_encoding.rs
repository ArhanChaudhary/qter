fn ans_encode(symbols: &[u16]) -> Vec<u8> {
    let probabilities = [2, 1, 1];

    let cdf = [0, 2, 3];

    let n = 2;

    let C = |state: u32, symbol: u16| {
        let p = probabilities[symbol as usize];
        ((state / p) << n) + (state % p) + cdf[symbol as usize]
    };

    let mut state = match symbols.last() {
        Some(last) => (0..(u16::MAX as u32))
            .find(|i| C(*i, *last) > u8::MAX as u32)
            .unwrap(),
        None => 0,
    };
    let mut stream = Vec::new();

    for symbol in symbols.iter().rev() {
        loop {
            let new_state = C(state, *symbol);

            if new_state.ilog2() >= 16 {
                stream.push((state & 0b11111111) as u8);
                state >>= 8;
            } else {
                state = new_state;
                break;
            }
        }
    }

    stream.extend_from_slice(&state.to_le_bytes()[0..2]);

    stream
}

fn ans_decode(mut data: &[u8]) -> Vec<u16> {
    let mut state = u16::from_le_bytes(TryFrom::try_from(&data[data.len() - 2..]).unwrap()) as u32;
    let mut output = Vec::new();

    data = &data[..data.len() - 2];

    let probabilities = [2, 1, 1];

    let cdf = [0, 2, 3];

    let n = 2;
    let mask = 0b11;

    let symbol_rev = [0, 0, 1, 2];

    'decoding: loop {
        let s = symbol_rev[(state & mask) as usize];
        output.push(s);

        state = probabilities[s as usize] * (state >> n) + (state & mask) - cdf[s as usize];

        if state.ilog2() < 8 {
            match data.split_last() {
                Some((v, new_data)) => {
                    data = new_data;

                    state <<= 8;
                    state |= *v as u32;
                }
                None => break 'decoding,
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use crate::table_encoding::ans_decode;

    use super::ans_encode;

    #[test]
    fn bruh() {
        let v = vec![0, 0, 1, 2, 1, 0, 0, 0, 2, 1, 0, 0, 0, 0, 2, 2, 0, 0, 1];
        let encoded = ans_encode(&v);
        let decoded = ans_decode(&encoded);

        assert_eq!(decoded, v);
    }
}
