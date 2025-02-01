pub struct PuzzleDef<S: Storage = HeapStorage> {
    pub name: String,
    pub orbit_defs: Vec<OrbitDef>,
    pub moves: Vec<Move<S>>,
}

pub struct StackStorage<const N: usize>;
pub struct HeapStorage;

pub trait Storage {
    type Buf: AsRef<[u8]> + AsMut<[u8]>;
}

impl<const N: usize> Storage for StackStorage<N> {
    type Buf = [u8; N];
}
impl Storage for HeapStorage {
    type Buf = Box<[u8]>;
}

#[repr(transparent)]
pub struct PuzzleState<S: Storage> {
    orbit_states: S::Buf,
}

pub struct Move<S: Storage> {
    // put this first to avoid the memory offset when dereferencing
    pub delta: PuzzleState<S>,
    pub name: String,
}

pub struct OrbitDef {
    pub size: u8,
    pub orientation_mod: u8,
    pub name: String,
}

impl<S: Storage> PuzzleDef<S> {
    fn get_move(&self, name: &str) -> Option<&Move<S>> {
        self.moves.iter().find(|def| def.name == name)
    }
}

pub trait PuzzleStateCore<S: Storage> {
    fn solved(orbit_defs: &[OrbitDef]) -> Self;
    fn from_orbit_states(orbit_states: S::Buf) -> Self;
    fn orbit_states(&self) -> &S::Buf;
    fn orbit_states_mut(&mut self) -> &mut S::Buf;

    fn replace_mul(&mut self, move_a: &Move<S>, move_b: &Move<S>, orbit_defs: &[OrbitDef]) {
        let a = move_a.delta.orbit_states.as_ref();
        let b = move_b.delta.orbit_states.as_ref();
        let orbit_states_mut = self.orbit_states_mut().as_mut();
        let mut base = 0;
        for &OrbitDef {
            size,
            orientation_mod,
            name: _,
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

impl<const N: usize> PuzzleStateCore<StackStorage<N>> for PuzzleState<StackStorage<N>> {
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

    fn from_orbit_states(orbit_states: [u8; N]) -> Self {
        PuzzleState { orbit_states }
    }

    fn orbit_states(&self) -> &[u8; N] {
        &self.orbit_states
    }

    fn orbit_states_mut(&mut self) -> &mut [u8; N] {
        &mut self.orbit_states
    }
}

impl PuzzleStateCore<HeapStorage> for PuzzleState<HeapStorage> {
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

    fn from_orbit_states(orbit_states: Box<[u8]>) -> Self {
        PuzzleState { orbit_states }
    }

    fn orbit_states(&self) -> &Box<[u8]> {
        &self.orbit_states
    }

    fn orbit_states_mut(&mut self) -> &mut Box<[u8]> {
        &mut self.orbit_states
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::phase2::defs::CUBE3_DEF;
    // use rstest::*;

    // #[fixture]
    // fn cube3_def() -> &'static PuzzleDef<Cube3Storage> {
    //     &*CUBE3_DEF
    // }

    // #[rstest]
    // fn test_composition(cube3_def: &PuzzleDef<Cube3Storage>) {
    //     let mut solved = PuzzleState::<Cube3Storage>::solved(&cube3_def.orbit_defs);
    //     let r_move = cube3_def.get_move("R").unwrap();
    //     let f_move = cube3_def.get_move("F").unwrap();
    //     solved.replace_mul(r_move, f_move, &cube3_def.orbit_defs);

    //     assert_eq!(
    //         solved.orbit_states(),
    //         &[
    //             9, 3, 7, 2, 1, 5, 6, 0, 8, 4, 10, 11, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 6, 1, 0,
    //             4, 2, 5, 3, 7, 2, 2, 2, 1, 1, 0, 1, 0,
    //         ][..]
    //     );
    // }

    // #[rstest]
    // fn test_composition_heap(cube3_def: &PuzzleDef<Cube3Storage>) {
    //     let mut solved = PuzzleState::<HeapStorage>::solved(&cube3_def.orbit_defs);
    //     let r_move = cube3_def.get_move("R").unwrap();
    //     let f_move = cube3_def.get_move("F").unwrap();
    //     solved.replace_mul(r_move, f_move, &cube3_def.orbit_defs);

    //     assert_eq!(
    //         solved.orbit_states(),
    //         &[
    //             9, 3, 7, 2, 1, 5, 6, 0, 8, 4, 10, 11, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 6, 1, 0,
    //             4, 2, 5, 3, 7, 2, 2, 2, 1, 1, 0, 1, 0,
    //         ][..]
    //     );
    // }
}
