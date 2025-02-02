pub trait PuzzleStorage {
    type Buf;
}

pub struct Move<S: PuzzleStorage> {
    // put this first to avoid the memory offset when dereferencing
    pub delta: PuzzleState<S>,
    pub name: String,
}

#[repr(transparent)]
pub struct PuzzleState<S: PuzzleStorage> {
    pub orbit_states: S::Buf,
}

pub trait SliceStorage {
    type Buf: AsRef<[u8]> + AsMut<[u8]> + for<'a> TryFrom<&'a [u8]>;
}

impl<S: SliceStorage> PuzzleStorage for S {
    type Buf = S::Buf;
}

pub struct OrbitDef {
    pub size: u8,
    pub orientation_mod: u8,
}

pub trait PuzzleStateInterface<S: PuzzleStorage> {
    fn solved(orbit_defs: &[OrbitDef]) -> Self;
    fn from_orbit_states(slice: &[u8]) -> Self;
    fn replace_compose(&mut self, move_a: &Move<S>, move_b: &Move<S>, orbit_defs: &[OrbitDef]);
}

impl<S: SliceStorage> PuzzleState<S> {
    pub fn replace_compose(&mut self, move_a: &Move<S>, move_b: &Move<S>, orbit_defs: &[OrbitDef]) {
        let a = move_a.delta.orbit_states.as_ref();
        let b = move_b.delta.orbit_states.as_ref();
        let orbit_states_mut = self.orbit_states.as_mut();
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
