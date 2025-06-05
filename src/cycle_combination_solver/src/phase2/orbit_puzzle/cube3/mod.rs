//! SIMD optimized implementations for 3x3 orbits during pruning table
//! generation.

use crate::phase2::puzzle::{PuzzleDef, PuzzleState};

pub mod avx2;
pub mod simd8and16;

// TODO: move to cube3
#[allow(clippy::missing_panics_doc)]
pub fn random_3x3_state<P: PuzzleState>(cube3_def: &PuzzleDef<P>, solved: &P) -> P {
    let mut result_1 = solved.clone();
    let mut result_2 = solved.clone();
    for _ in 0..20 {
        let move_index = fastrand::choice(0_u8..18).unwrap();
        let move_ = &cube3_def.moves[move_index as usize];
        // SAFETY: the arguments correspond to `sorted_orbit_defs`
        unsafe {
            result_1.replace_compose(&result_2, &move_.puzzle_state, &cube3_def.sorted_orbit_defs);
        }
        std::mem::swap(&mut result_2, &mut result_1);
    }
    result_2
}
