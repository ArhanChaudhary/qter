#![warn(clippy::pedantic)]

use core::{fmt::Debug, hash::Hash};
use std::{cell::RefCell, collections::HashMap, iter::Sum, marker::PhantomData, rc::Rc};

use num_traits::{NumAssign, NumCast, PrimInt, ToBytes};

trait Seal {}

pub trait State: TakeFrom + core::fmt::Display + Debug + Sum + NumAssign {
    type NextDown: TakeFrom;
    const RANGE_SIZE: Self;
    const RANGE_BITS: u32;

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
            const RANGE_BITS: u32 = <$next_down>::BITS;

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

pub trait CodingFSM<S: State>: Debug {
    fn symbol_count(&self) -> usize;

    fn found_symbol(&mut self, symbol: usize);

    fn predict_next_symbol(&self, out: &mut [S]);
}

pub trait ReversibleFSM<S: State>: CodingFSM<S> {
    fn uncall_found_symbol(&mut self, symbol: usize);
    // if only we were coding in janus xD
}

#[derive(Clone)]
pub struct Cache<S: State, FSM: CodingFSM<S> + Eq + Hash + Clone> {
    fsm: FSM,
    // Allow being cloned over and over again inside a `MakeReversible`
    cache: Rc<RefCell<HashMap<FSM, Vec<S>>>>,
}

impl<S: State, FSM: CodingFSM<S> + Eq + Hash + Clone> Debug for Cache<S, FSM> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fsm.fmt(f)
    }
}

impl<S: State, FSM: CodingFSM<S> + Eq + Hash + Clone> Cache<S, FSM> {
    pub fn new(fsm: FSM) -> Self {
        let mut cache = Cache {
            fsm,
            cache: Rc::new(RefCell::new(HashMap::new())),
        };
        cache.cache_current_prediction();
        cache
    }

    fn cache_current_prediction(&mut self) {
        let mut data = vec![S::zero(); self.fsm.symbol_count()];
        self.fsm.predict_next_symbol(&mut data);
        self.cache.borrow_mut().insert(self.fsm.to_owned(), data);
    }
}

impl<S: State, FSM: CodingFSM<S> + Eq + Hash + Clone> CodingFSM<S> for Cache<S, FSM> {
    fn symbol_count(&self) -> usize {
        self.fsm.symbol_count()
    }

    fn found_symbol(&mut self, symbol: usize) {
        self.fsm.found_symbol(symbol);

        if !self.cache.borrow().contains_key(&self.fsm) {
            self.cache_current_prediction();
        }
    }

    fn predict_next_symbol(&self, out: &mut [S]) {
        let cache = self.cache.borrow();
        let prediction = cache
            .get(&self.fsm)
            .expect("The predictions to be cached after calling `found_symbol`");
        out.copy_from_slice(prediction);
    }
}

impl<S: State, FSM: ReversibleFSM<S> + Eq + Hash + Clone> ReversibleFSM<S> for Cache<S, FSM> {
    fn uncall_found_symbol(&mut self, symbol: usize) {
        self.fsm.uncall_found_symbol(symbol);
    }
}

struct MakeReversible<S: State, FSM: CodingFSM<S> + Clone> {
    current_fsm: FSM,
    stack: Vec<FSM>,
    phantom: PhantomData<S>,
}

impl<S: State, FSM: CodingFSM<S> + Clone> Debug for MakeReversible<S, FSM> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.current_fsm.fmt(f)
    }
}

impl<S: State, FSM: CodingFSM<S> + Clone> MakeReversible<S, FSM> {
    fn new(fsm: FSM) -> Self {
        MakeReversible {
            current_fsm: fsm,
            stack: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<S: State, FSM: CodingFSM<S> + Clone> CodingFSM<S> for MakeReversible<S, FSM> {
    fn symbol_count(&self) -> usize {
        self.current_fsm.symbol_count()
    }

    fn found_symbol(&mut self, symbol: usize) {
        let prev_fsm = self.current_fsm.to_owned();
        self.current_fsm.found_symbol(symbol);
        self.stack.push(prev_fsm);
    }

    fn predict_next_symbol(&self, out: &mut [S]) {
        self.current_fsm.predict_next_symbol(out);
    }
}

impl<S: State, FSM: CodingFSM<S> + Clone> ReversibleFSM<S> for MakeReversible<S, FSM> {
    fn uncall_found_symbol(&mut self, _: usize) {
        self.current_fsm = match self.stack.pop() {
            Some(v) => v,
            None => unreachable!(),
        };
    }
}

fn coding_function<S: State, T: Debug>(
    state: S,
    symbol: usize,
    ranges: &[S],
    fsm: &T,
) -> Option<S> {
    let p = ranges[symbol];

    assert!(
        (p != S::zero()),
        "Got a symbol with range zero: {symbol}; {ranges:?}; {fsm:?}"
    );

    let divided = state / p;

    // Doing the bitshift will make the number fall out of range
    if divided.leading_zeros() < S::RANGE_BITS {
        return None;
    }

    Some(
        (divided << S::RANGE_BITS as usize)
            + (state % p)
            + ranges.iter().take(symbol).copied().sum::<S>(),
    )
}

pub fn ans_encode<S: State, FSM: CodingFSM<S> + Clone>(
    stream: &mut Vec<u8>,
    symbols: &[usize],
    initial_state: FSM,
) {
    let mut reversible = MakeReversible::new(initial_state);

    // We don't want to do the last symbol
    for symbol in &symbols[0..symbols.len() - 1] {
        reversible.found_symbol(*symbol);
    }

    ans_encode_inplace(stream, symbols, reversible);
}

/// Encodes the symbols in the stream
///
/// # Panics
///
/// Panics if the symbol is too large for the range
pub fn ans_encode_inplace<S: State, FSM: ReversibleFSM<S>>(
    stream: &mut Vec<u8>,
    symbols: &[usize],
    mut final_state: FSM,
) {
    let symbol_count = final_state.symbol_count();

    let mut last_ranges = vec![S::zero(); symbol_count];
    final_state.predict_next_symbol(&mut last_ranges);

    let (mut state, mut symbols) = match symbols.split_last() {
        Some((last, symbols)) => {
            let mut i = S::zero();
            loop {
                if let Some(code) = coding_function(i, *last, &last_ranges, &final_state) {
                    if code > S::RANGE_SIZE - S::one() {
                        break (code, symbols);
                    }
                }
                i += S::one();

                assert!(
                    i <= S::RANGE_SIZE,
                    "The symbol {last} is too large for the range {i}; {last_ranges:?}; {final_state:?}"
                );
            }
        }
        None => (S::zero(), symbols),
    };

    let starts_at = stream.len();

    while let Some((symbol, prev)) = symbols.split_last() {
        final_state.uncall_found_symbol(*symbol);
        let mut ranges = vec![S::zero(); symbol_count];
        final_state.predict_next_symbol(&mut ranges);

        loop {
            if let Some(new_state) = coding_function(state, *symbol, &ranges, &final_state) {
                state = new_state;
                break;
            }
            stream.extend_from_slice(
                (<S::NextDown as NumCast>::from(state & (S::RANGE_SIZE - S::one())))
                    .unwrap()
                    .to_be_bytes()
                    .as_ref(),
            );
            state = state >> S::RANGE_BITS as usize;
        }

        symbols = prev;
    }

    stream.extend_from_slice(state.to_be_bytes().as_ref());

    stream[starts_at..].reverse();
}

pub fn ans_decode<S: State, FSM: CodingFSM<S>>(
    data: &mut impl Iterator<Item = u8>,
    max_symbols: Option<usize>,
    mut fsm: FSM,
) -> Option<Vec<usize>> {
    if let Some(max) = max_symbols {
        if max == 0 {
            return Some(vec![]);
        }
    }

    let symbol_count = fsm.symbol_count();

    let mut ranges = vec![S::zero(); symbol_count];
    fsm.predict_next_symbol(&mut ranges);

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

        state = ranges[symbol] * (state >> S::RANGE_BITS as usize) + (state & mask) - cdf_val;

        while state == S::zero() || state.ilog2() < S::RANGE_BITS {
            if let Some(v) = S::NextDown::take_from(data) {
                state = (state << S::RANGE_BITS as usize) | S::from(v).unwrap();
            } else {
                break 'decoding;
            }
        }

        fsm.found_symbol(symbol);
        fsm.predict_next_symbol(&mut ranges);
    }

    Some(output)
}

#[cfg(test)]
mod tests {
    use crate::{Cache, CodingFSM, ans_decode, ans_encode};

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct Fsm {
        prev: Option<usize>,
    }

    impl CodingFSM<u16> for Fsm {
        fn symbol_count(&self) -> usize {
            3
        }

        fn found_symbol(&mut self, symbol: usize) {
            self.prev = Some(symbol);
        }

        fn predict_next_symbol(&self, out: &mut [u16]) {
            let dist = match self.prev {
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
        }
    }

    #[test]
    fn test_encoding() {
        let v = [
            0, 1, 0, 2, 0, 2, 1, 0, 1, 0, 2, 0, 2, 0, 1, 2, 0, 2, 0, 1, 0, 1, 2, 0,
        ];

        let mut encoded = Vec::new();
        ans_encode(&mut encoded, &v, Fsm { prev: None });
        println!("{encoded:?}");
        let decoded = ans_decode(&mut encoded.iter().copied(), None, Fsm { prev: None }).unwrap();
        assert_eq!(decoded, v);
        encoded.extend_from_slice(&[1, 2, 3, 4, 5]);
        let decoded = ans_decode(
            &mut encoded.iter().copied(),
            Some(v.len()),
            Fsm { prev: None },
        )
        .unwrap();
        assert_eq!(decoded, v);
    }

    #[test]
    fn test_caching() {
        let v = [
            0, 1, 0, 2, 0, 2, 1, 0, 1, 0, 2, 0, 2, 0, 1, 2, 0, 2, 0, 1, 0, 1, 2, 0,
        ];

        let mut encoded = Vec::new();
        ans_encode(&mut encoded, &v, Cache::new(Fsm { prev: None }));
        println!("{encoded:?}");
        let decoded = ans_decode(
            &mut encoded.iter().copied(),
            None,
            Cache::new(Fsm { prev: None }),
        )
        .unwrap();
        assert_eq!(decoded, v);
    }
}
