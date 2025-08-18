#![feature(test, slice_index_methods, portable_simd, abi_vectorcall)]
#![warn(clippy::pedantic)]
#![allow(clippy::similar_names, clippy::too_many_lines)]

pub(crate) mod canonical_fsm;
pub(crate) mod orbit_puzzle;
pub(crate) mod permutator;
pub mod pruning;
pub mod puzzle;
pub(crate) mod puzzle_state_history;
pub mod solver;
pub use generativity::*;

#[macro_export]
macro_rules! start {
    ($msg:expr) => {
        concat!("⏳ ", $msg)
    };
}

#[macro_export]
macro_rules! working {
    ($msg:expr) => {
        concat!("🛠  ", $msg)
    };
}

#[macro_export]
macro_rules! success {
    ($msg:expr) => {
        concat!("✅ ", $msg)
    };
}

pub trait SliceView {
    type Slice<'a>
    where
        Self: 'a;

    fn slice_view(&self) -> Self::Slice<'_>;
}
pub trait SliceViewMut {
    type SliceMut<'a>
    where
        Self: 'a;

    fn slice_view_mut(&mut self) -> Self::SliceMut<'_>;
}

pub trait Rebrand<'id> {
    type Rebranded<'id2>: Rebrand<'id2>;

    fn rebrand<'id2>(self, id: Id<'id2>) -> Self::Rebranded<'id2>;
}

// We can do one more however it will overflow when adding more to it which is
// common in context
const FACT_UNTIL_19: [u64; 20] = {
    let mut arr = [0; 20];
    arr[0] = 1;
    let mut i = 1;
    while i < arr.len() {
        arr[i] = arr[i - 1] * i as u64;
        i += 1;
    }
    arr
};
