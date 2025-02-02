use qter_core::phase2_puzzle::{
    Move, OrbitDef, PuzzleState, PuzzleStateInterface, PuzzleStorage, SliceStorage,
};
use std::marker::PhantomData;

pub struct StackPuzzle<const N: usize>;
pub struct HeapPuzzle;
pub struct SimdPuzzle<T>(PhantomData<T>);

impl<const N: usize> SliceStorage for StackPuzzle<N> {
    type Buf = [u8; N];
}

impl SliceStorage for HeapPuzzle {
    type Buf = Box<[u8]>;
}

impl<T> PuzzleStorage for SimdPuzzle<T> {
    type Buf = T;
}

pub trait PuzzleStateInterfaceSlice<S: SliceStorage>: PuzzleStateInterface<S> {
    fn orbit_states(&self) -> &S::Buf;
    fn orbit_states_mut(&mut self) -> &mut S::Buf;

    fn replace_compose(&mut self, move_a: &Move<S>, move_b: &Move<S>, orbit_defs: &[OrbitDef]) {
        let a = move_a.delta.orbit_states.as_ref();
        let b = move_b.delta.orbit_states.as_ref();
        let orbit_states_mut = self.orbit_states_mut().as_mut();
        let mut base = 0;
        for &OrbitDef {
            size,
            orientation_mod,
        } in orbit_defs
        {
            let size = size as usize;
            if orientation_mod > 1 {
                for i in 0..size {
                    let base_i = base + i;
                    unsafe {
                        let pos = a.get_unchecked(base + *b.get_unchecked(base_i) as usize);
                        let a_ori =
                            a.get_unchecked(base + *b.get_unchecked(base_i) as usize + size);
                        let b_ori = b.get_unchecked(base_i + size);
                        *orbit_states_mut.get_unchecked_mut(base_i) = *pos;
                        *orbit_states_mut.get_unchecked_mut(base_i + size) =
                            (*a_ori + *b_ori) % orientation_mod;
                    }
                }
            } else {
                for i in 0..size {
                    let base_i = base + i;
                    unsafe {
                        let pos = *a.get_unchecked(base + *b.get_unchecked(base_i) as usize);
                        *orbit_states_mut.get_unchecked_mut(base_i) = pos;
                        *orbit_states_mut.get_unchecked_mut(base_i + size) = 0;
                    }
                }
            }
            base += size * 2;
        }
    }
}

impl<S: SliceStorage> PuzzleStateInterfaceSlice<S> for PuzzleState<S>
where
    PuzzleState<S>: PuzzleStateInterface<S>,
{
    fn orbit_states(&self) -> &S::Buf {
        &self.orbit_states
    }

    fn orbit_states_mut(&mut self) -> &mut S::Buf {
        &mut self.orbit_states
    }
}

impl<const N: usize> PuzzleStateInterface<StackPuzzle<N>> for PuzzleState<StackPuzzle<N>> {
    fn solved(orbit_defs: &[OrbitDef]) -> Self {
        let mut orbit_states = [0_u8; N];
        let mut base = 0;
        for &OrbitDef { size, .. } in orbit_defs.iter() {
            for j in 1..size {
                orbit_states[base as usize + j as usize] = j;
            }
            base += 2 * size;
        }
        PuzzleState { orbit_states }
    }

    fn from_orbit_states(slice: &[u8]) -> Self {
        PuzzleState {
            orbit_states: slice.try_into().unwrap(),
        }
    }

    fn replace_compose(
        &mut self,
        move_a: &Move<StackPuzzle<N>>,
        move_b: &Move<StackPuzzle<N>>,
        orbit_defs: &[OrbitDef],
    ) {
        <Self as PuzzleStateInterfaceSlice<StackPuzzle<N>>>::replace_compose(
            self, move_a, move_b, orbit_defs,
        );
    }
}

impl PuzzleStateInterface<HeapPuzzle> for PuzzleState<HeapPuzzle> {
    fn solved(orbit_defs: &[OrbitDef]) -> Self {
        let mut orbit_states = vec![0_u8; orbit_defs.iter().map(|def| def.size as usize * 2).sum()];
        let mut base = 0;
        for &OrbitDef { size, .. } in orbit_defs.iter() {
            for j in 1..size {
                orbit_states[base as usize + j as usize] = j;
            }
            base += 2 * size;
        }
        PuzzleState {
            orbit_states: orbit_states.into_boxed_slice(),
        }
    }

    fn from_orbit_states(slice: &[u8]) -> Self {
        PuzzleState {
            orbit_states: slice.into(),
        }
    }

    fn replace_compose(
        &mut self,
        move_a: &Move<HeapPuzzle>,
        move_b: &Move<HeapPuzzle>,
        orbit_defs: &[OrbitDef],
    ) {
        <Self as PuzzleStateInterfaceSlice<HeapPuzzle>>::replace_compose(
            self, move_a, move_b, orbit_defs,
        );
    }
}

pub struct PuzzleDef<S: PuzzleStorage> {
    pub name: String,
    pub orbit_defs: Vec<OrbitDef>,
    pub moves: Vec<Move<S>>,
}

impl<S: PuzzleStorage> PuzzleDef<S> {
    fn get_move(&self, name: &str) -> Option<&Move<S>> {
        self.moves.iter().find(|def| def.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::defs::cube3_def;
    use rstest::*;

    fn compose_r_f<S>() -> PuzzleState<S>
    where
        S: PuzzleStorage,
        PuzzleState<S>: PuzzleStateInterface<S>,
    {
        // let mut solved = PuzzleState::<StackPuzzle<CUBE3_STACK>>::solved(&cube3_def.orbit_defs);
        let cube3_def = cube3_def::<S>();
        let mut solved = PuzzleState::solved(&cube3_def.orbit_defs);
        let r_move = cube3_def.get_move("R").unwrap();
        let f_move = cube3_def.get_move("F").unwrap();
        PuzzleStateInterface::replace_compose(&mut solved, r_move, f_move, &cube3_def.orbit_defs);
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
        assert_eq!(compose_actual.orbit_states(), compose_expected);
    }

    #[rstest]
    fn test_composition_heap(compose_expected: &[u8]) {
        let compose_actual = compose_r_f::<HeapPuzzle>();
        assert_eq!(
            compose_actual.orbit_states().iter().as_slice(),
            compose_expected
        );
    }
}
