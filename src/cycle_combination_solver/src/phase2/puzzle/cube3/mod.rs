#[cfg(not(any(simd32, simd8and16)))]
pub type Cube3 = StackPuzzle<40>;

#[cfg(any(simd32, simd8and16))]
mod common {
    use crate::phase2::puzzle::OrbitDef;
    use std::sync::LazyLock;

    pub static CUBE_3_SORTED_ORBIT_DEFS: LazyLock<Vec<OrbitDef>> = LazyLock::new(|| {
        vec![
            OrbitDef {
                piece_count: 8.try_into().unwrap(),
                orientation_count: 3.try_into().unwrap(),
            },
            OrbitDef {
                piece_count: 12.try_into().unwrap(),
                orientation_count: 2.try_into().unwrap(),
            },
        ]
    });
}

mod simd32;
mod simd8and16;

#[cfg(all(not(simd32), simd8and16))]
pub use simd8and16::Cube3;

#[cfg(simd32)]
pub use simd32::Cube3;
