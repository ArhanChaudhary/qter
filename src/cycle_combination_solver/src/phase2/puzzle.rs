use std::hint::unreachable_unchecked;

pub struct PuzzleDef<S: Storage = HeapStorage> {
    pub name: String,
    pub orbit_defs: Vec<OrbitDef>,
    pub moves: Vec<Move<S>>,
}

pub type Cube3Storage = StackStorage<40>;
pub type Cube4Storage = StackStorage<112>;
pub type LargePuzzleStorage = StackStorage<256>;
pub type LargerPuzzleStorage = StackStorage<512>;

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

enum OrbitStateStorage<S: Storage> {
    Stack(S::Buf),
    Heap(S::Buf),
}

#[repr(transparent)]
pub struct PuzzleState<S: Storage> {
    orbit_states: OrbitStateStorage<S>,
}

pub struct Move<S: Storage> {
    // put this first to avoid the memory offset when dereferencing
    pub r#move: PuzzleState<S>,
    pub name: String,
}

pub struct OrbitDef {
    pub size: u8,
    pub orientation_mod: u8,
    pub name: String,
}

impl<S: Storage> PuzzleDef<S> {
    fn get_move(&self, name: &str) -> Option<&Move<S>> {
        self.r#moves.iter().find(|def| def.name == name)
    }
}

pub trait PuzzleStateCore<S: Storage>
where
    PuzzleState<S>: PuzzleStateCore<S>,
{
    fn solved(orbit_defs: &[OrbitDef]) -> Self;
    fn from_orbit_states(orbit_states: S::Buf) -> Self;
    fn orbit_states(&self) -> &S::Buf;
    fn orbit_states_mut(&mut self) -> &mut S::Buf;

    fn replace_mul(&mut self, move_a: &Move<S>, move_b: &Move<S>, orbit_defs: &[OrbitDef]) {
        let a = move_a.r#move.orbit_states().as_ref();
        let b = move_b.r#move.orbit_states().as_ref();
        let mut base = 0;
        for &OrbitDef {
            name: _,
            size,
            orientation_mod,
        } in orbit_defs
        {
            let size = size as usize;
            if orientation_mod > 1 {
                for j in 0..size {
                    unsafe {
                        let pos = a.get_unchecked(base + *b.get_unchecked(base + j) as usize);
                        let a_ori =
                            a.get_unchecked(base + *b.get_unchecked(base + j) as usize + size);
                        let b_ori = b.get_unchecked(base + j + size);
                        *self.orbit_states_mut().as_mut().get_unchecked_mut(base + j) = *pos;
                        *self
                            .orbit_states_mut()
                            .as_mut()
                            .get_unchecked_mut(base + j + size) =
                            (*a_ori + *b_ori) % orientation_mod;
                    }
                }
            } else {
                for j in 0..size {
                    unsafe {
                        let pos = *a.get_unchecked(base + *b.get_unchecked(base + j) as usize);
                        *self.orbit_states_mut().as_mut().get_unchecked_mut(base + j) = pos;
                        *self
                            .orbit_states_mut()
                            .as_mut()
                            .get_unchecked_mut(base + j + size) = 0;
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
            for j in 0..size {
                orbit_states[base as usize + j as usize] = j;
            }
            base += 2 * size;
        }
        PuzzleState {
            orbit_states: OrbitStateStorage::Stack(orbit_states),
        }
    }

    fn from_orbit_states(orbit_states: [u8; N]) -> Self {
        PuzzleState {
            orbit_states: OrbitStateStorage::Stack(orbit_states),
        }
    }

    fn orbit_states(&self) -> &[u8; N] {
        match &self.orbit_states {
            OrbitStateStorage::Stack(orbit_states) => orbit_states,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn orbit_states_mut(&mut self) -> &mut [u8; N] {
        match &mut self.orbit_states {
            OrbitStateStorage::Stack(orbit_states) => orbit_states,
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

impl PuzzleStateCore<HeapStorage> for PuzzleState<HeapStorage> {
    fn solved(orbit_defs: &[OrbitDef]) -> Self {
        let mut orbit_states = Vec::new();
        for &OrbitDef { size, .. } in orbit_defs.iter() {
            for j in 0..size {
                orbit_states.push(j);
            }
        }
        PuzzleState {
            orbit_states: OrbitStateStorage::Heap(orbit_states.into_boxed_slice()),
        }
    }

    fn from_orbit_states(orbit_states: Box<[u8]>) -> Self {
        PuzzleState {
            orbit_states: OrbitStateStorage::Heap(orbit_states),
        }
    }

    fn orbit_states(&self) -> &Box<[u8]> {
        match &self.orbit_states {
            OrbitStateStorage::Heap(orbit_states) => orbit_states,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn orbit_states_mut(&mut self) -> &mut Box<[u8]> {
        match &mut self.orbit_states {
            OrbitStateStorage::Heap(orbit_states) => orbit_states,
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::defs::CUBE3_DEF;

    #[test]
    fn test_composition() {
        let cube3_def = &*CUBE3_DEF;
        let mut solved = PuzzleState::<Cube3Storage>::solved(&cube3_def.orbit_defs);
        let r_move = cube3_def.get_move("R").unwrap();
        let f_move = cube3_def.get_move("F").unwrap();
        solved.replace_mul(r_move, f_move, &cube3_def.orbit_defs);

        assert_eq!(
            solved.orbit_states(),
            &[
                9, 3, 7, 2, 1, 5, 6, 0, 8, 4, 10, 11, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 6, 1, 0,
                4, 2, 5, 3, 7, 2, 2, 2, 1, 1, 0, 1, 0,
            ][..]
        );
    }
}
