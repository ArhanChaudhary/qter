use super::{
    KSolveConversionError, MultiBvInterface, OrbitDef, OrbitPuzzleState, OrientedPartition,
    PuzzleState,
};

#[derive(Clone, PartialEq, Debug)]
pub struct SliceOrbitPuzzleState(Box<[u8]>);

impl OrbitPuzzleState for SliceOrbitPuzzleState {
    fn new_solved_state() -> Self {
        todo!();
    }
}
