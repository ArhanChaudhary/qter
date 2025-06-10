use crate::architectures::{Algorithm, PermutationGroup};
use crate::{Int, U, WithSpan};
use std::fmt::Debug;
use std::sync::Arc;

/// The facelets needed for `solved-goto`
#[derive(Debug, Clone)]
pub struct Facelets(pub Vec<usize>);

/// The generator of a register along with the facelets needed to decode it
pub struct RegisterGenerator;

impl SeparatesByPuzzleType for RegisterGenerator {
    type Theoretical<'s> = ();

    type Puzzle<'s> = (Algorithm, Facelets);
}

pub trait SeparatesByPuzzleType {
    type Theoretical<'s>;

    type Puzzle<'s>;
}

#[derive(Clone)]
pub enum ByPuzzleType<'a, T: SeparatesByPuzzleType> {
    Theoretical(T::Theoretical<'a>),
    Puzzle(T::Puzzle<'a>),
}

impl<'a, T: SeparatesByPuzzleType> Debug for ByPuzzleType<'a, T>
where
    T::Theoretical<'a>: Debug,
    T::Puzzle<'a>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ByPuzzleType::Theoretical(v) => {
                f.debug_tuple("ByPuzzleType::Theoretical").field(v).finish()
            }
            ByPuzzleType::Puzzle(v) => f.debug_tuple("ByPuzzleType::Puzzle").field(v).finish(),
        }
    }
}

/// A qter instruction
#[derive(Debug)]
pub enum Instruction {
    Goto { instruction_idx: usize },
    SolvedGoto(ByPuzzleType<'static, SolvedGoto>),
    Input(ByPuzzleType<'static, Input>),
    Halt(ByPuzzleType<'static, Halt>),
    Print(ByPuzzleType<'static, Print>),
    PerformAlgorithm(ByPuzzleType<'static, PerformAlgorithm>),
}

#[derive(Clone, Debug)]
pub struct SolvedGoto {
    pub instruction_idx: usize,
    pub puzzle_idx: usize,
}

impl SeparatesByPuzzleType for SolvedGoto {
    type Theoretical<'s> = Self;

    type Puzzle<'s> = (Self, Facelets);
}

#[derive(Clone, Debug)]
pub struct Input {
    pub message: String,
    pub puzzle_idx: usize,
}

impl SeparatesByPuzzleType for Input {
    type Theoretical<'s> = Self;

    type Puzzle<'s> = (Self, Algorithm, Facelets);
}

#[derive(Clone, Debug)]
pub struct Halt {
    pub message: String,
}

impl SeparatesByPuzzleType for Halt {
    type Theoretical<'s> = (Self, Option<usize>);

    type Puzzle<'s> = (Self, Option<(usize, Algorithm, Facelets)>);
}

#[derive(Clone, Debug)]
pub struct Print {
    pub message: String,
}

impl SeparatesByPuzzleType for Print {
    type Theoretical<'s> = (Self, Option<usize>);

    type Puzzle<'s> = (Self, Option<(usize, Algorithm, Facelets)>);
}

#[derive(Clone, Debug)]
pub struct PerformAlgorithm {
    pub puzzle_idx: usize,
}

impl SeparatesByPuzzleType for PerformAlgorithm {
    type Theoretical<'s> = (Self, Int<U>);

    type Puzzle<'s> = (Self, Algorithm);
}

/// A qter program
#[derive(Debug)]
pub struct Program {
    /// A list of theoretical registers along with their orders
    pub theoretical: Vec<WithSpan<Int<U>>>,
    /// A list of puzzles to be used for registers
    pub puzzles: Vec<WithSpan<Arc<PermutationGroup>>>,
    /// The program itself
    pub instructions: Vec<WithSpan<Instruction>>,
}
