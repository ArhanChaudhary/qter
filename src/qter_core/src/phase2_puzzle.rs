pub struct Move<S: PuzzleStateInterface> {
    // put this first to avoid the memory offset when dereferencing (note from henry: this probably doesn't matter because of effective addressing, but if you find that I'm wrong you should use #[repr(C)] to prevent rust from reordering them)
    pub delta: S,
    pub name: String,
}

pub struct OrbitDef {
    pub size: u8,
    pub orientation_mod: u8,
}

pub trait PuzzleStateInterface {
    fn solved(orbit_defs: &[OrbitDef]) -> Self;
    fn from_orbit_states(slice: &[u8]) -> Self;
    fn replace_compose(&mut self, move_a: &Self, move_b: &Self, orbit_defs: &[OrbitDef]);
}

pub fn slice_replace_compose(
    orbit_states_mut: &mut [u8],
    a: &[u8],
    b: &[u8],
    orbit_defs: &[OrbitDef],
) {
    debug_assert_eq!(
        orbit_defs
            .iter()
            .map(|v| (v.size as usize) * 2)
            .sum::<usize>(),
        orbit_states_mut.len()
    );
    debug_assert_eq!(orbit_states_mut.len(), a.len());
    debug_assert_eq!(a.len(), b.len());

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
                    let a_ori = a.get_unchecked(base + *b.get_unchecked(base_i) as usize + size);
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
