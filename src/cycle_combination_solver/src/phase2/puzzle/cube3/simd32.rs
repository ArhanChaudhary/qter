#![cfg_attr(not(simd32), allow(dead_code, unused_variables))]

use super::common::Cube3Interface;
use crate::phase2::puzzle::OrientedPartition;
use std::{
    hash::{Hash, Hasher},
    simd::u8x32,
};

#[derive(Clone, Debug)]
pub struct Cube3(u8x32);

impl PartialEq for Cube3 {
    fn eq(&self, other: &Self) -> bool {
        todo!();
    }
}

impl Hash for Cube3 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        todo!();
    }
}

impl Cube3Interface for Cube3 {
    fn from_sorted_transformations_unchecked(sorted_transformations: &[Vec<(u8, u8)>]) -> Self {
        todo!();
    }

    fn replace_compose(&mut self, a: &Self, b: &Self) {
        todo!();
    }

    fn replace_inverse(&mut self, a: &Self) {
        todo!();
    }

    fn induces_sorted_cycle_type(
        &self,
        sorted_cycle_type: &[OrientedPartition],
        multi_bv: [u16; 2],
    ) -> bool {
        todo!();
    }
}

// pub struct StackEvenCubeSimd<const S_24S: usize> {
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32; S_24S],
// }

// pub struct HeapEvenCubeSimd {
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32],
// }

// pub struct StackOddCubeSimd<const S_24S: usize> {
//     ep: u8x16,
//     eo: u8x16,
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32; S_24S],
// }

// pub struct HeapOddCubeSimd {
//     cp: u8x8,
//     co: u8x8,
//     s_24s: [u8x32],
// }
