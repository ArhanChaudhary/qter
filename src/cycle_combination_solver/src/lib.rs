#![feature(
    test,
    slice_index_methods,
    portable_simd,
    abi_vectorcall,
    try_reserve_kind
)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::inline_always,
    refining_impl_trait_reachable
)]

mod phase1;
pub mod phase2;
