use qter_core::phase2_puzzle::{slice_replace_compose, Move, OrbitDef, PuzzleStateInterface};

pub struct StackPuzzle<const N: usize>([u8; N]);
pub struct HeapPuzzle(Box<[u8]>);

impl<const N: usize> PuzzleStateInterface for StackPuzzle<N> {
    fn solved(orbit_defs: &[OrbitDef]) -> Self {
        let mut orbit_states = [0_u8; N];
        let mut base = 0;
        for &OrbitDef { size, .. } in orbit_defs.iter() {
            for j in 1..size {
                orbit_states[base as usize + j as usize] = j;
            }
            base += 2 * size;
        }
        StackPuzzle(orbit_states)
    }

    fn from_orbit_states(slice: &[u8]) -> Self {
        StackPuzzle(slice.try_into().unwrap())
    }

    fn replace_compose(
        &mut self,
        move_a: &StackPuzzle<N>,
        move_b: &StackPuzzle<N>,
        orbit_defs: &[OrbitDef],
    ) {
        slice_replace_compose(&mut self.0, &move_a.0, &move_b.0, orbit_defs);
    }
}

impl PuzzleStateInterface for HeapPuzzle {
    fn solved(orbit_defs: &[OrbitDef]) -> Self {
        let mut orbit_states = vec![0_u8; orbit_defs.iter().map(|def| def.size as usize * 2).sum()];
        let mut base = 0;
        for &OrbitDef { size, .. } in orbit_defs.iter() {
            for j in 1..size {
                orbit_states[base as usize + j as usize] = j;
            }
            base += 2 * size;
        }
        HeapPuzzle(orbit_states.into_boxed_slice())
    }

    fn from_orbit_states(slice: &[u8]) -> Self {
        HeapPuzzle(slice.into())
    }

    fn replace_compose(
        &mut self,
        move_a: &HeapPuzzle,
        move_b: &HeapPuzzle,
        orbit_defs: &[OrbitDef],
    ) {
        slice_replace_compose(&mut self.0, &move_a.0, &move_b.0, orbit_defs);
    }
}

// struct SupportedSimd<const N: usize>(Simd<u8, N>);

// impl<const N: usize> Deref for SupportedSimd<N> {
//     type Target = Simd<u8, N>;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

pub fn supports_dyn_shuffle<const N: usize>() -> bool {
    match N {
        #[cfg(all(
            any(
                target_arch = "aarch64",
                target_arch = "arm64ec",
                all(target_arch = "arm", target_feature = "v7")
            ),
            target_feature = "neon",
            target_endian = "little"
        ))]
        8 => true,
        #[cfg(target_feature = "ssse3")]
        16 => true,
        #[cfg(target_feature = "simd128")]
        16 => true,
        #[cfg(all(
            any(target_arch = "aarch64", target_arch = "arm64ec"),
            target_feature = "neon",
            target_endian = "little"
        ))]
        16 => true,
        #[cfg(all(target_feature = "avx2", not(target_feature = "avx512vbmi")))]
        32 => true,
        #[cfg(all(target_feature = "avx512vl", target_feature = "avx512vbmi"))]
        32 => true,
        // #[cfg(target_feature = "avx512vbmi")]
        // 64 => transize(x86::_mm512_permutexvar_epi8, self, idxs),
        _ => false,
    }
}

pub struct PuzzleDef<S: PuzzleStateInterface> {
    pub name: String,
    pub orbit_defs: Vec<OrbitDef>,
    pub moves: Vec<Move<S>>,
}

impl<S: PuzzleStateInterface> PuzzleDef<S> {
    fn get_move(&self, name: &str) -> Option<&Move<S>> {
        self.moves.iter().find(|def| def.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::defs::{cube3_def, Cube3PuzzleSimd};
    use rstest::*;

    fn compose_r_f<S: PuzzleStateInterface>() -> S {
        let cube3_def = cube3_def::<S>();
        let mut solved = S::solved(&cube3_def.orbit_defs);
        let r_move = cube3_def.get_move("R").unwrap();
        let f_move = cube3_def.get_move("F").unwrap();
        solved.replace_compose(&r_move.delta, &f_move.delta, &cube3_def.orbit_defs);
        solved
    }

    #[fixture]
    fn compose_expected() -> &'static [u8] {
        &[
            9, 3, 7, 2, 1, 5, 6, 0, 8, 4, 10, 11, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 6, 1, 0, 4,
            2, 5, 3, 7, 2, 2, 2, 1, 1, 0, 1, 0,
        ]
    }

    #[rstest]
    fn test_composition_stack(compose_expected: &[u8]) {
        let compose_actual = compose_r_f::<StackPuzzle<40>>();
        assert_eq!(compose_actual.0, compose_expected);
    }

    #[rstest]
    fn test_composition_heap(compose_expected: &[u8]) {
        let compose_actual = compose_r_f::<HeapPuzzle>();
        assert_eq!(compose_actual.0.iter().as_slice(), compose_expected);
    }

    #[rstest]
    fn test_composition_simd(compose_expected: &[u8]) {
        let compose_actual = compose_r_f::<Cube3PuzzleSimd>();
        println!("{:?}, {:?}", compose_actual.ep, compose_actual.cp);
        println!("{:?}", compose_expected);
        // assert_eq!(compose_actual.orbit_states, compose_expected);
    }
}
