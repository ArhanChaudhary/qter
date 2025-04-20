#![feature(test)]
#![feature(slice_index_methods)]
#![feature(portable_simd)]
#![feature(abi_vectorcall)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::inline_always,
    refining_impl_trait_reachable
)]

mod phase1;
pub mod phase2;
