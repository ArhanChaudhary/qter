#[repr(C)]
pub struct Move<P: PuzzleState> {
    pub delta: P,
    pub name: String,
}

pub struct OrbitDef {
    pub size: u8,
    pub orientation_mod: u8,
}

pub trait PuzzleState {
    type PuzzleMeta;

    fn solved(puzzle_meta: &Self::PuzzleMeta) -> Self;
    fn from_orbit_states(slice: &[u8]) -> Self;
    fn replace_compose(&mut self, move_a: &Self, move_b: &Self, puzzle_meta: &Self::PuzzleMeta);
}
