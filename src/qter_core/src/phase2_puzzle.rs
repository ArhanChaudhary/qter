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
